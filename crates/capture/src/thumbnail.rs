use jpeg_encoder::{ColorType, Encoder};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CAPTUREBLT, CreateCompatibleDC, CreateDIBSection,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetMonitorInfoW, HBITMAP, HDC, HGDIOBJ,
    HMONITOR, MONITORINFO, ReleaseDC, SRCCOPY, SelectObject,
};
use windows::Win32::Storage::Xps::{PRINT_WINDOW_FLAGS, PrintWindow};
use windows::Win32::UI::WindowsAndMessaging::{GetWindowRect, IsIconic};

/// The picker draws thumbnails at most 240 CSS pixels wide; this stays sharp
/// up to 150% display scaling.
const MAX_WIDTH: u32 = 360;
const MAX_HEIGHT: u32 = 203;

const QUALITY: u8 = 80;

/// Asks the window to render its full content, including `DirectComposition`
/// and hardware-accelerated surfaces.
const PW_RENDERFULLCONTENT: PRINT_WINDOW_FLAGS = PRINT_WINDOW_FLAGS(2);

/// A still image of the window as a JPEG, or `None` when the window is
/// minimized or renders nothing.
///
/// `PrintWindow` asks the window to paint into a bitmap, so unlike Windows
/// Graphics Capture it draws no capture border around the window.
pub fn window_thumbnail(hwnd: HWND) -> Option<Vec<u8>> {
    if unsafe { IsIconic(hwnd).as_bool() } {
        return None;
    }

    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &raw mut rect) }.ok()?;
    let width = u32::try_from(rect.right - rect.left)
        .ok()
        .filter(|w| *w > 0)?;
    let height = u32::try_from(rect.bottom - rect.top)
        .ok()
        .filter(|h| *h > 0)?;

    let bgra = render(width, height, |memory, _| unsafe {
        PrintWindow(hwnd, memory, PW_RENDERFULLCONTENT).as_bool()
    })?;
    if bgra
        .as_chunks::<4>()
        .0
        .iter()
        .all(|pixel| pixel[..3] == [0, 0, 0])
    {
        return None;
    }

    let (small, small_width, small_height) = downscale(&bgra, width, height, MAX_WIDTH, MAX_HEIGHT);

    encode_jpeg(&small, small_width, small_height)
}

/// A still image of a whole monitor as a JPEG, copied from the screen.
pub(crate) fn monitor_thumbnail(hmonitor: HMONITOR) -> Option<Vec<u8>> {
    let mut info = MONITORINFO {
        cbSize: u32::try_from(size_of::<MONITORINFO>()).ok()?,
        ..Default::default()
    };
    if !unsafe { GetMonitorInfoW(hmonitor, &raw mut info) }.as_bool() {
        return None;
    }

    let rect = info.rcMonitor;
    let width = u32::try_from(rect.right - rect.left)
        .ok()
        .filter(|w| *w > 0)?;
    let height = u32::try_from(rect.bottom - rect.top)
        .ok()
        .filter(|h| *h > 0)?;

    let bgra = render(width, height, |memory, screen| unsafe {
        BitBlt(
            memory,
            0,
            0,
            rect.right - rect.left,
            rect.bottom - rect.top,
            Some(screen),
            rect.left,
            rect.top,
            SRCCOPY | CAPTUREBLT,
        )
        .is_ok()
    })?;

    let (small, small_width, small_height) = downscale(&bgra, width, height, MAX_WIDTH, MAX_HEIGHT);

    encode_jpeg(&small, small_width, small_height)
}

/// Lets `paint` draw into a `width` x `height` BGRA bitmap, given the
/// bitmap DC and the screen DC, and returns the pixels when it succeeds.
fn render(width: u32, height: u32, paint: impl FnOnce(HDC, HDC) -> bool) -> Option<Vec<u8>> {
    let screen = ScreenDc(unsafe { GetDC(None) });
    let memory = MemoryDc(unsafe { CreateCompatibleDC(Some(screen.0)) });

    let header = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: u32::try_from(size_of::<BITMAPINFOHEADER>()).ok()?,
            biWidth: i32::try_from(width).ok()?,
            biHeight: -i32::try_from(height).ok()?,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bits = std::ptr::null_mut();
    let bitmap = unsafe {
        CreateDIBSection(
            Some(memory.0),
            &raw const header,
            DIB_RGB_COLORS,
            &raw mut bits,
            None,
            0,
        )
    }
    .ok()?;
    let bitmap = Bitmap(bitmap);

    let previous = unsafe { SelectObject(memory.0, bitmap.0.into()) };
    let painted = paint(memory.0, screen.0);
    let pixels = painted.then(|| {
        let len = width as usize * height as usize * 4;

        unsafe { std::slice::from_raw_parts(bits.cast::<u8>(), len) }.to_vec()
    });
    unsafe { SelectObject(memory.0, previous) };

    pixels
}

/// Averages square blocks of pixels down to fit `max_width` x `max_height`,
/// keeping the aspect ratio.
fn downscale(
    bgra: &[u8],
    width: u32,
    height: u32,
    max_width: u32,
    max_height: u32,
) -> (Vec<u8>, u32, u32) {
    let factor = width
        .div_ceil(max_width)
        .max(height.div_ceil(max_height))
        .max(1);
    let small_width = (width / factor).max(1);
    let small_height = (height / factor).max(1);

    let mut small = Vec::with_capacity(small_width as usize * small_height as usize * 4);
    for y in 0..small_height {
        for x in 0..small_width {
            let mut sum = [0u32; 4];
            for dy in 0..factor {
                for dx in 0..factor {
                    let index = (((y * factor + dy) * width + x * factor + dx) * 4) as usize;
                    for (channel, total) in sum.iter_mut().enumerate() {
                        *total += u32::from(bgra[index + channel]);
                    }
                }
            }

            let count = factor * factor;
            small.extend(sum.map(|total| u8::try_from(total / count).unwrap_or(u8::MAX)));
        }
    }

    (small, small_width, small_height)
}

fn encode_jpeg(bgra: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    let mut jpeg = Vec::new();
    let encoder = Encoder::new(&mut jpeg, QUALITY);
    encoder
        .encode(
            bgra,
            u16::try_from(width).ok()?,
            u16::try_from(height).ok()?,
            ColorType::Bgra,
        )
        .ok()?;

    Some(jpeg)
}

struct ScreenDc(HDC);

impl Drop for ScreenDc {
    fn drop(&mut self) {
        unsafe { ReleaseDC(None, self.0) };
    }
}

struct MemoryDc(HDC);

impl Drop for MemoryDc {
    fn drop(&mut self) {
        let _ = unsafe { DeleteDC(self.0) };
    }
}

struct Bitmap(HBITMAP);

impl Drop for Bitmap {
    fn drop(&mut self) {
        let _ = unsafe { DeleteObject(HGDIOBJ::from(self.0)) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downscale_fits_the_maximum_and_keeps_the_aspect_ratio() {
        let bgra = vec![255; 1920 * 1080 * 4];

        let (small, width, height) = downscale(&bgra, 1920, 1080, MAX_WIDTH, MAX_HEIGHT);

        assert_eq!((width, height), (320, 180));
        assert_eq!(small.len(), 320 * 180 * 4);
    }

    #[test]
    fn downscale_averages_each_block() {
        // One 2x2 block: two black pixels and two white ones.
        let bgra = [
            0, 0, 0, 0, 255, 255, 255, 255, 0, 0, 0, 0, 255, 255, 255, 255,
        ];

        let (small, width, height) = downscale(&bgra, 2, 2, 1, 1);

        assert_eq!((width, height), (1, 1));
        assert_eq!(small, vec![127, 127, 127, 127]);
    }

    #[test]
    fn a_small_window_keeps_its_size() {
        let bgra = vec![10; 100 * 50 * 4];

        let (_, width, height) = downscale(&bgra, 100, 50, MAX_WIDTH, MAX_HEIGHT);

        assert_eq!((width, height), (100, 50));
    }
}
