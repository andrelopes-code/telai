use windows::Win32::System::SystemInformation::{GetVersionExW, OSVERSIONINFOW};

/// First Windows 11 build: the Mica material exists from here on.
const FIRST_MICA_BUILD: u32 = 22000;

#[allow(
    clippy::cast_possible_truncation,
    reason = "the structure is a few hundred bytes"
)]
const INFO_SIZE: u32 = size_of::<OSVERSIONINFOW>() as u32;

/// Whether Windows can paint the Mica material behind the app window.
///
/// The app manifest declares Windows 10 support, so `GetVersionExW` reports
/// the real build number instead of a compatibility value.
#[must_use]
pub fn supports_mica() -> bool {
    let mut info = OSVERSIONINFOW {
        dwOSVersionInfoSize: INFO_SIZE,
        ..Default::default()
    };

    #[allow(deprecated, reason = "the replacement needs the driver kit headers")]
    let read = unsafe { GetVersionExW(&raw mut info) };

    read.is_ok() && info.dwBuildNumber >= FIRST_MICA_BUILD
}
