use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::PathBuf;

use telai_capture::ProcessTree;

/// Seconds of silence before the duplicate-audio warning clears. Quiet
/// passages would otherwise make it flicker.
const SILENT_CHECKS: u32 = 3;

/// Whether the browser that shows the player also plays its audio aloud.
#[derive(Default)]
pub struct BrowserAudio {
    audible: bool,
    silent_checks: u32,
}

impl BrowserAudio {
    pub fn update(&mut self, viewers: &[SocketAddr], server_port: u16, source_process: u32) {
        self.record(browser_playing(viewers, server_port, source_process));
    }

    #[must_use]
    pub fn audible(&self) -> bool {
        self.audible
    }

    fn record(&mut self, playing: bool) {
        if playing {
            self.audible = true;
            self.silent_checks = 0;
            return;
        }

        self.silent_checks += 1;
        if self.silent_checks >= SILENT_CHECKS {
            self.audible = false;
        }
    }
}

/// Matches the executables of the viewer connections against the processes
/// that play sound. The captured app is left out: it plays its own audio.
fn browser_playing(viewers: &[SocketAddr], server_port: u16, source_process: u32) -> bool {
    let browsers: HashSet<PathBuf> = viewers
        .iter()
        .filter_map(|viewer| match viewer {
            SocketAddr::V4(address) => telai_capture::tcp_client_process(*address, server_port),
            SocketAddr::V6(_) => None,
        })
        .filter_map(telai_capture::image_path)
        .collect();
    if browsers.is_empty() {
        return false;
    }

    let (Ok(audible), Ok(tree)) = (telai_capture::audible_processes(), ProcessTree::snapshot())
    else {
        return false;
    };

    audible
        .into_iter()
        .filter(|process| !tree.is_within(*process, source_process))
        .filter_map(telai_capture::image_path)
        .any(|path| browsers.contains(&path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_turns_the_warning_on_at_once() {
        let mut browser = BrowserAudio::default();

        browser.record(true);

        assert!(browser.audible());
    }

    #[test]
    fn the_warning_clears_after_three_silent_checks() {
        let mut browser = BrowserAudio::default();
        browser.record(true);

        browser.record(false);
        browser.record(false);
        assert!(browser.audible());

        browser.record(false);
        assert!(!browser.audible());
    }
}
