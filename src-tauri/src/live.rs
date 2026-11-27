use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use telai_capture::{AudioCapture, Frame, VideoCapture, VideoSink, WindowInfo};
use telai_stream::EncoderPool;
use telai_stream::protocol::Status;

use crate::app::AppState;
use crate::browser_audio::BrowserAudio;
use crate::commands::{CommandError, icon_data_url};
use crate::tray;

pub const LIVE_EVENT: &str = "live";
pub const ENDED_EVENT: &str = "live-ended";

/// Fast enough for a smooth level meter.
const TICK: Duration = Duration::from_millis(100);

/// Window and browser checks enumerate processes and audio sessions, so they
/// run once per second instead of every tick.
const SLOW_CHECK_TICKS: u32 = 10;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// What the window shows while streaming.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveSnapshot {
    pub source: SourceView,
    /// Unix time in milliseconds.
    pub started_at: u64,
    pub viewers: usize,
    /// The player tab plays the audio aloud on this PC, doubling the sound.
    pub browser_audible: bool,
    pub minimized: bool,
    /// Peak audio level since the previous snapshot, from 0 to 1.
    pub level: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceView {
    pub handle: isize,
    pub title: String,
    pub process_name: String,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EndReason {
    WindowClosed,
    CaptureFailed,
}

/// A running capture of one window and its process audio.
pub struct LiveSession {
    id: u64,
    video: Option<VideoCapture>,
    audio: Option<AudioCapture>,
    shared: Arc<Shared>,
}

/// State written by the capture threads and read by the monitor.
struct Shared {
    source: SourceView,
    process_id: u32,
    started_at: u64,
    /// Bit pattern of the peak level, as `f32`. Non-negative floats order the
    /// same way as their bits, so `fetch_max` works on them.
    level: AtomicU32,
    window_closed: AtomicBool,
    stopped: AtomicBool,
}

impl LiveSession {
    fn snapshot(&self, viewers: usize, browser_audible: bool, minimized: bool) -> LiveSnapshot {
        let level = f32::from_bits(self.shared.level.swap(0, Ordering::Relaxed));

        LiveSnapshot {
            source: self.shared.source.clone(),
            started_at: self.shared.started_at,
            viewers,
            browser_audible,
            minimized,
            level,
        }
    }

    fn is_finished(&self) -> bool {
        self.video.as_ref().is_none_or(VideoCapture::is_finished)
    }
}

impl Drop for LiveSession {
    fn drop(&mut self) {
        self.shared.stopped.store(true, Ordering::Relaxed);

        if let Some(audio) = self.audio.take() {
            audio.stop();
        }
        if let Some(video) = self.video.take() {
            video.stop();
        }
    }
}

pub fn is_live(app: &AppHandle) -> bool {
    lock_live(app).is_some()
}

pub fn snapshot(app: &AppHandle) -> Option<LiveSnapshot> {
    lock_live(app)
        .as_ref()
        .map(|session| session.snapshot(0, false, false))
}

/// Replaces any running session with a capture of the window `handle`.
pub fn start(app: &AppHandle, handle: isize) -> Result<LiveSnapshot, CommandError> {
    stop(app);

    let window = telai_capture::list_windows()
        .map_err(CommandError::video)?
        .into_iter()
        .find(|window| window.handle == handle)
        .ok_or(CommandError::WindowGone)?;

    let session = open_session(app, window)?;
    let snapshot = session.snapshot(0, false, false);
    let id = session.id;
    let shared = Arc::clone(&session.shared);

    *lock_live(app) = Some(session);
    tray::set_live(app, true);

    let monitor_app = app.clone();
    thread::Builder::new()
        .name("live-monitor".into())
        .spawn(move || monitor(&monitor_app, id, &shared))
        .map_err(|error| CommandError::Internal(error.to_string()))?;

    Ok(snapshot)
}

pub fn stop(app: &AppHandle) {
    let session = lock_live(app).take();
    if session.is_none() {
        return;
    }

    drop(session);
    go_offline(app);
}

fn open_session(app: &AppHandle, window: WindowInfo) -> Result<LiveSession, CommandError> {
    let state = app.state::<AppState>();
    let started_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        });

    let shared = Arc::new(Shared {
        source: SourceView {
            handle: window.handle,
            title: window.title.clone(),
            process_name: window.process_name.clone(),
            icon: window.icon_png.as_deref().map(icon_data_url),
        },
        process_id: window.process_id,
        started_at,
        level: AtomicU32::new(0),
        window_closed: AtomicBool::new(false),
        stopped: AtomicBool::new(false),
    });

    let audio_hub = state.hub.clone();
    let audio_shared = Arc::clone(&shared);
    let audio = AudioCapture::start(window.process_id, move |samples| {
        audio_hub.publish_audio(samples);

        let peak = samples
            .iter()
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
        audio_shared
            .level
            .fetch_max(peak.min(1.0).to_bits(), Ordering::Relaxed);
    })
    .map_err(CommandError::audio)?;

    let sink = EncoderSink {
        pool: EncoderPool::new(state.hub.clone()),
        shared: Arc::clone(&shared),
    };
    let video = VideoCapture::start(window.handle, sink).map_err(CommandError::video)?;

    state.hub.set_status(Status {
        live: true,
        title: window.title,
    });

    Ok(LiveSession {
        id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        video: Some(video),
        audio: Some(audio),
        shared,
    })
}

/// Reports the session to the window until it stops, and ends it when the
/// captured window goes away.
fn monitor(app: &AppHandle, id: u64, shared: &Shared) {
    let state = app.state::<AppState>();
    let mut tick = 0;
    let mut minimized = false;
    let mut browser = BrowserAudio::default();

    loop {
        thread::sleep(TICK);
        if shared.stopped.load(Ordering::Relaxed) {
            return;
        }

        let viewers = state.hub.viewers().borrow().clone();
        if tick % SLOW_CHECK_TICKS == 0 {
            minimized = telai_capture::is_minimized(shared.source.handle);
            browser.update(&viewers, state.port, shared.process_id);
        }
        tick += 1;

        let live = lock_live(app);
        let Some(session) = live.as_ref().filter(|session| session.id == id) else {
            return;
        };

        if session.is_finished() {
            drop(live);
            let reason = if shared.window_closed.load(Ordering::Relaxed) {
                EndReason::WindowClosed
            } else {
                EndReason::CaptureFailed
            };
            end(app, id, reason);
            return;
        }

        let snapshot = session.snapshot(viewers.len(), browser.audible(), minimized);
        drop(live);
        let _ = app.emit(LIVE_EVENT, snapshot);
    }
}

fn end(app: &AppHandle, id: u64, reason: EndReason) {
    let session = {
        let mut live = lock_live(app);
        if live.as_ref().is_none_or(|session| session.id != id) {
            return;
        }
        live.take()
    };

    drop(session);
    go_offline(app);
    let _ = app.emit(ENDED_EVENT, reason);
}

fn go_offline(app: &AppHandle) {
    app.state::<AppState>().hub.set_status(Status::default());
    tray::set_live(app, false);
}

fn lock_live(app: &AppHandle) -> MutexGuard<'_, Option<LiveSession>> {
    app.state::<AppState>()
        .inner()
        .live
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

struct EncoderSink {
    pool: EncoderPool,
    shared: Arc<Shared>,
}

impl VideoSink for EncoderSink {
    fn frame(&mut self, frame: &Frame<'_>) {
        let mut bgra = self.pool.buffer();
        frame.copy_to(&mut bgra);
        self.pool.submit(frame.width, frame.height, bgra);
    }

    fn closed(&mut self) {
        self.shared.window_closed.store(true, Ordering::Relaxed);
    }
}
