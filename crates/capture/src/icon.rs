use std::collections::HashMap;
use std::iter;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, PoisonError};

use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC,
    DeleteObject, GetDIBits, GetObjectW, HBITMAP, HDC,
};
use windows::Win32::UI::Shell::SHDefExtractIconW;
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};
use windows::core::PCWSTR;

/// Shown at 24 CSS pixels, so this stays sharp up to 200% display scaling.
const ICON_SIZE: u32 = 48;

/// Icons by executable path. The window list refreshes every few seconds, and
/// an executable keeps its icon while it runs.
static ICONS: LazyLock<Mutex<HashMap<PathBuf, Option<Vec<u8>>>>> = LazyLock::new(Mutex::default);

/// The executable's main icon as a PNG image.
pub fn executable_icon_png(path: &Path) -> Option<Vec<u8>> {
    let mut icons = ICONS.lock().unwrap_or_else(PoisonError::into_inner);

    icons
        .entry(path.to_path_buf())
        .or_insert_with(|| extract_icon_png(path))
        .clone()
}

fn extract_icon_png(path: &Path) -> Option<Vec<u8>> {
    let wide_path: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect();

    let mut icon = HICON::default();
    let large = Some(&raw mut icon);
    let extracted =
        unsafe { SHDefExtractIconW(PCWSTR(wide_path.as_ptr()), 0, 0, large, None, ICON_SIZE) };
    if extracted.is_err() || icon.is_invalid() {
        return None;
    }

    let icon = OwnedIcon(icon);
    let (width, height, rgba) = icon_rgba(icon.0)?;

    encode_png(width, height, &rgba)
}

fn icon_rgba(icon: HICON) -> Option<(u32, u32, Vec<u8>)> {
    let mut info = ICONINFO::default();
    unsafe { GetIconInfo(icon, &raw mut info) }.ok()?;
    let color = OwnedBitmap(info.hbmColor);
    let mask = OwnedBitmap(info.hbmMask);
    if color.0.is_invalid() {
        return None;
    }

    let (width, height) = bitmap_size(color.0)?;
    let context = OwnedDc(unsafe { CreateCompatibleDC(None) });

    let mut pixels = read_bgra(context.0, color.0, width, height)?;
    let has_alpha = pixels.as_chunks::<4>().0.iter().any(|pixel| pixel[3] != 0);
    if !has_alpha {
        // Icons without an alpha channel mark transparency in the mask instead:
        // a white mask pixel is transparent.
        let mask_pixels = read_bgra(context.0, mask.0, width, height)?;
        let mask_pixels = mask_pixels.as_chunks::<4>().0;
        for (pixel, mask_pixel) in pixels.as_chunks_mut::<4>().0.iter_mut().zip(mask_pixels) {
            pixel[3] = if mask_pixel[0] == 0 { u8::MAX } else { 0 };
        }
    }

    for pixel in pixels.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }

    Some((width, height, pixels))
}

fn bitmap_size(bitmap: HBITMAP) -> Option<(u32, u32)> {
    let mut description = BITMAP::default();
    let size = i32::try_from(size_of::<BITMAP>()).ok()?;
    let written = unsafe { GetObjectW(bitmap.into(), size, Some((&raw mut description).cast())) };
    if written == 0 {
        return None;
    }

    Some((
        u32::try_from(description.bmWidth).ok()?,
        u32::try_from(description.bmHeight).ok()?,
    ))
}

/// Reads a bitmap as top-down 32-bit BGRA rows.
fn read_bgra(context: HDC, bitmap: HBITMAP, width: u32, height: u32) -> Option<Vec<u8>> {
    let mut header = BITMAPINFO {
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

    let mut pixels = vec![0u8; width as usize * height as usize * 4];
    let lines = unsafe {
        GetDIBits(
            context,
            bitmap,
            0,
            height,
            Some(pixels.as_mut_ptr().cast()),
            &raw mut header,
            DIB_RGB_COLORS,
        )
    };

    (lines == i32::try_from(height).ok()?).then_some(pixels)
}

fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Option<Vec<u8>> {
    let mut png = Vec::new();

    let mut encoder = png::Encoder::new(&mut png, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder.write_header().ok()?;
    writer.write_image_data(rgba).ok()?;
    writer.finish().ok()?;

    Some(png)
}

struct OwnedIcon(HICON);

impl Drop for OwnedIcon {
    fn drop(&mut self) {
        let _ = unsafe { DestroyIcon(self.0) };
    }
}

struct OwnedBitmap(HBITMAP);

impl Drop for OwnedBitmap {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            let _ = unsafe { DeleteObject(self.0.into()) };
        }
    }
}

struct OwnedDc(HDC);

impl Drop for OwnedDc {
    fn drop(&mut self) {
        let _ = unsafe { DeleteDC(self.0) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_icon_of_a_system_executable_is_a_png() {
        let explorer = Path::new(r"C:\Windows\explorer.exe");

        let png = executable_icon_png(explorer).expect("explorer has an icon");

        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    }
}
