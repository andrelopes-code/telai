use std::collections::HashMap;
use std::ffi::c_void;
use std::path::PathBuf;

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
use windows::Win32::UI::WindowsAndMessaging::{GetClientRect, IsIconic, IsWindow};
use windows_capture::window::Window;

use crate::process::{ProcessTree, image_path};
use crate::sessions::audible_processes;
use crate::{Error, icon};

/// `DWMWA_CLOAKED` writes one `u32`.
const CLOAKED_SIZE: u32 = 4;

/// A top-level window that can be captured.
#[derive(Debug, Clone)]
pub struct WindowInfo {
    /// The `HWND`, as an integer.
    pub handle: isize,
    pub title: String,
    pub process_id: u32,
    /// Executable name without its extension.
    pub process_name: String,
    pub icon_png: Option<Vec<u8>>,
    /// Whether the window's process tree is playing sound right now.
    pub playing_audio: bool,
}

/// Visible windows of other processes, front to back.
pub fn list_windows() -> Result<Vec<WindowInfo>, Error> {
    let windows = Window::enumerate().map_err(|error| Error::Video(error.to_string()))?;
    let tree = ProcessTree::snapshot()?;
    let audible = audible_processes().unwrap_or_else(|error| {
        tracing::warn!(%error, "audio sessions are unavailable");
        Vec::new()
    });

    let own_process = std::process::id();
    let mut executables: HashMap<u32, Option<PathBuf>> = HashMap::new();
    let mut icons: HashMap<PathBuf, Option<Vec<u8>>> = HashMap::new();

    let mut listed = Vec::new();
    for window in windows {
        let Ok(process_id) = window.process_id() else {
            continue;
        };
        let title = window.title().unwrap_or_default();
        let hwnd = HWND(window.as_raw_hwnd());
        if process_id == own_process || title.trim().is_empty() || !is_shown(hwnd) {
            continue;
        }

        let executable = executables
            .entry(process_id)
            .or_insert_with(|| image_path(process_id));
        let process_name = executable
            .as_deref()
            .and_then(|path| path.file_stem())
            .map_or_else(
                || window.process_name().unwrap_or_default(),
                |stem| stem.to_string_lossy().into_owned(),
            );
        let icon_png = executable.as_ref().and_then(|path| {
            icons
                .entry(path.clone())
                .or_insert_with(|| icon::executable_icon_png(path))
                .clone()
        });

        listed.push(WindowInfo {
            handle: hwnd.0 as isize,
            title,
            process_id,
            process_name,
            icon_png,
            playing_audio: audible
                .iter()
                .any(|&audible| tree.is_within(audible, process_id)),
        });
    }

    Ok(listed)
}

/// Whether the window still exists and is minimized. A minimized window
/// produces no new frames.
#[must_use]
pub fn is_minimized(handle: isize) -> bool {
    let hwnd = to_hwnd(handle);

    unsafe { IsWindow(Some(hwnd)).as_bool() && IsIconic(hwnd).as_bool() }
}

pub(crate) fn to_hwnd(handle: isize) -> HWND {
    HWND(handle as *mut c_void)
}

/// Filters out windows that are hidden by the shell (cloaked, such as
/// suspended Store apps) or that have no client area. Minimized windows are
/// kept: they render again once restored.
fn is_shown(hwnd: HWND) -> bool {
    let mut cloaked = 0u32;
    let attribute = unsafe {
        DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, (&raw mut cloaked).cast(), CLOAKED_SIZE)
    };
    if attribute.is_ok() && cloaked != 0 {
        return false;
    }

    if unsafe { IsIconic(hwnd).as_bool() } {
        return true;
    }

    let mut client = RECT::default();
    let has_area = unsafe { GetClientRect(hwnd, &raw mut client) }.is_ok();

    has_area && client.right > client.left && client.bottom > client.top
}
