/// First Windows 11 build with `DWMWA_SYSTEMBACKDROP_TYPE`: from here on the
/// Acrylic material blurs the windows behind without lag on drag or resize.
const FIRST_ACRYLIC_BUILD: u32 = 22523;

/// Whether Windows can paint the Acrylic material behind the app window.
///
/// `windows-version` reads the build with `RtlGetVersion`. `GetVersionExW`
/// reports build 9200 to an app whose manifest declares no `supportedOS`,
/// and the manifest Tauri embeds declares none.
#[must_use]
pub fn supports_acrylic() -> bool {
    windows_version::OsVersion::current().build >= FIRST_ACRYLIC_BUILD
}
