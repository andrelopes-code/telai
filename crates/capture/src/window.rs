use std::collections::HashMap;
use std::ffi::c_void;
use std::path::{Path, PathBuf};

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, GetClientRect, GetWindowThreadProcessId, IsIconic, IsWindow,
};
use windows::core::{PCWSTR, w};
use windows_capture::window::Window;

use crate::process::{ProcessTree, image_path};
use crate::sessions::audible_processes;
use crate::{Error, icon, thumbnail};

/// `DWMWA_CLOAKED` writes one `u32`.
const CLOAKED_SIZE: u32 = 4;

/// Hosts the windows of Store (UWP) apps. The app itself, and its sound, run
/// in a separate process that is not a child of the host.
const FRAME_HOST: &str = "ApplicationFrameHost.exe";

/// Class of the window, inside the host's frame, that the app draws into.
const CORE_WINDOW: PCWSTR = w!("Windows.UI.Core.CoreWindow");

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
    /// A still of the window as a JPEG; `None` while it is minimized.
    pub thumbnail_jpeg: Option<Vec<u8>>,
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

    let listed = windows
        .iter()
        .filter_map(|window| {
            let mut process_id = window.process_id().ok().filter(|&id| id != own_process)?;
            if executable_of(&mut executables, process_id).is_some_and(is_frame_host) {
                let frame = HWND(window.as_raw_hwnd());
                process_id = hosted_process(process_id, core_window_process(frame));
            }

            describe(window, process_id, executable_of(&mut executables, process_id))
        })
        .map(|mut info| {
            info.playing_audio = audible
                .iter()
                .any(|&process| tree.is_within(process, info.process_id));
            info
        })
        .collect();

    Ok(listed)
}

fn executable_of(cache: &mut HashMap<u32, Option<PathBuf>>, process_id: u32) -> Option<&Path> {
    cache
        .entry(process_id)
        .or_insert_with(|| image_path(process_id))
        .as_deref()
}

fn is_frame_host(executable: &Path) -> bool {
    executable
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case(FRAME_HOST))
}

/// The process of the app inside a Store app frame. While the app is
/// minimized or suspended its core window leaves the frame, and the host
/// stays the owner.
fn hosted_process(host: u32, core_window: Option<u32>) -> u32 {
    core_window
        .filter(|&id| id != 0 && id != host)
        .unwrap_or(host)
}

/// Owner of the core window that a Store app frame contains, if any.
fn core_window_process(frame: HWND) -> Option<u32> {
    let core = unsafe { FindWindowExW(Some(frame), None, CORE_WINDOW, PCWSTR::null()) };
    let core = core.ok()?;

    let mut process_id = 0;
    unsafe { GetWindowThreadProcessId(core, Some(&raw mut process_id)) };

    Some(process_id)
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

/// A shown, titled window, without its audio state.
fn describe(window: &Window, process_id: u32, executable: Option<&Path>) -> Option<WindowInfo> {
    let title = window.title().ok()?;
    let hwnd = HWND(window.as_raw_hwnd());
    if title.trim().is_empty() || !is_shown(hwnd) {
        return None;
    }

    let process_name = executable.and_then(Path::file_stem).map_or_else(
        || window.process_name().unwrap_or_default(),
        |stem| stem.to_string_lossy().into_owned(),
    );

    Some(WindowInfo {
        handle: hwnd.0 as isize,
        title,
        process_id,
        process_name,
        icon_png: executable.and_then(icon::executable_icon_png),
        thumbnail_jpeg: thumbnail::window_thumbnail(hwnd),
        playing_audio: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_store_app_frame_host_is_recognized_in_any_case() {
        let host = Path::new(r"C:\WINDOWS\system32\ApplicationFrameHost.exe");
        let lower_case = Path::new(r"C:\Windows\System32\applicationframehost.EXE");
        let explorer = Path::new(r"C:\Windows\explorer.exe");

        assert!(is_frame_host(host));
        assert!(is_frame_host(lower_case));
        assert!(!is_frame_host(explorer));
    }

    #[test]
    fn a_framed_store_app_is_owned_by_its_core_window_process() {
        assert_eq!(hosted_process(10_084, Some(23_788)), 23_788);
    }

    #[test]
    fn a_frame_without_a_core_window_stays_with_its_host() {
        assert_eq!(hosted_process(10_084, None), 10_084);
        assert_eq!(hosted_process(10_084, Some(0)), 10_084);
        assert_eq!(hosted_process(10_084, Some(10_084)), 10_084);
    }
}
