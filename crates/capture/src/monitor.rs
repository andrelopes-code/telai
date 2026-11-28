use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, HMONITOR, MONITORINFO};
use windows::Win32::UI::WindowsAndMessaging::MONITORINFOF_PRIMARY;
use windows_capture::monitor::Monitor;

use crate::{Error, thumbnail};

/// A display that can be captured whole.
#[derive(Debug, Clone)]
pub struct MonitorInfo {
    /// The `HMONITOR`, as an integer.
    pub handle: isize,
    /// One-based position in the order Windows lists the displays.
    pub number: usize,
    /// The display's model name; empty when Windows does not report one.
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub primary: bool,
    /// A still of the whole display as a JPEG.
    pub thumbnail_jpeg: Option<Vec<u8>>,
}

/// Every connected display, in the order Windows lists them.
pub fn list_monitors() -> Result<Vec<MonitorInfo>, Error> {
    let monitors = Monitor::enumerate().map_err(|error| Error::Video(error.to_string()))?;

    let listed = monitors
        .iter()
        .enumerate()
        .filter_map(|(index, monitor)| {
            let hmonitor = HMONITOR(monitor.as_raw_hmonitor());

            Some(MonitorInfo {
                handle: hmonitor.0 as isize,
                number: index + 1,
                name: monitor.name().unwrap_or_default(),
                width: monitor.width().ok()?,
                height: monitor.height().ok()?,
                primary: is_primary(hmonitor),
                thumbnail_jpeg: thumbnail::monitor_thumbnail(hmonitor),
            })
        })
        .collect();

    Ok(listed)
}

fn is_primary(hmonitor: HMONITOR) -> bool {
    let mut info = MONITORINFO {
        cbSize: u32::try_from(size_of::<MONITORINFO>()).unwrap_or_default(),
        ..Default::default()
    };

    unsafe { GetMonitorInfoW(hmonitor, &raw mut info) }.as_bool()
        && info.dwFlags & MONITORINFOF_PRIMARY != 0
}
