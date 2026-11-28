//! Window video and per-process audio capture on Windows.
//!
//! Video comes from Windows Graphics Capture, audio from WASAPI process
//! loopback, which records only the audio rendered by one process tree.

#![cfg(windows)]

mod audio;
mod icon;
mod mixer;
mod monitor;
mod net;
mod process;
mod sessions;
mod system;
mod thumbnail;
mod video;
mod window;

pub use audio::AudioCapture;
pub use monitor::{MonitorInfo, list_monitors};
pub use net::tcp_client_process;
pub use process::{ProcessTree, image_path};
pub use sessions::{AudioApp, audible_processes, audio_apps};
pub use system::supports_acrylic;
pub use video::{CaptureTarget, Frame, VideoCapture, VideoSink};
pub use window::{WindowInfo, is_minimized, list_windows};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("window capture failed: {0}")]
    Video(String),
    #[error("audio capture failed: {0}")]
    Audio(#[from] wasapi::WasapiError),
    #[error("the audio capture thread exited during startup")]
    AudioThreadExited,
    #[error("could not start a capture thread: {0}")]
    Thread(#[from] std::io::Error),
    #[error(transparent)]
    Windows(#[from] windows::core::Error),
}
