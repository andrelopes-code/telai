use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use telai_capture::{
    AudioApp, AudioCapture, CaptureTarget, Frame, MixedAudioCapture, MonitorInfo, VideoCapture,
    VideoSink, WindowInfo,
};
use telai_stream::EncoderPool;
use telai_stream::protocol::Status;

use crate::app::AppState;
use crate::audio_prefs::AudioPrefs;
use crate::browser_audio::BrowserAudio;
use crate::commands::{CommandError, icon_data_url};
use crate::text::text;
use crate::tray;

pub const LIVE_EVENT: &str = "live";
pub const ENDED_EVENT: &str = "live-ended";

/// Fast enough for a smooth level meter.
const TICK: Duration = Duration::from_millis(100);

/// Window, browser and audio app checks enumerate processes and audio
/// sessions, so they run once per second instead of every tick.
const SLOW_CHECK_TICKS: u32 = 10;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// What the user picked to stream.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SourceRef {
    Window { handle: isize },
    Monitor { handle: isize },
}

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
    /// The apps whose sound a screen stream can carry; `None` for a window,
    /// which carries only its own sound.
    pub audio_apps: Option<Vec<AudioAppView>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceKind {
    Window,
    Monitor,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceView {
    pub kind: SourceKind,
    pub handle: isize,
    pub title: String,
    pub process_name: String,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioAppView {
    pub process_id: u32,
    pub name: String,
    pub icon: Option<String>,
    pub included: bool,
    /// From 0 to 1.
    pub volume: f32,
    /// Peak level since the previous snapshot, after the volume, from 0 to 1.
    pub level: f32,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EndReason {
    WindowClosed,
    CaptureFailed,
}

/// A running capture of one window and its process audio, or of a whole
/// monitor and the apps the user chose.
pub struct LiveSession {
    id: u64,
    video: Option<VideoCapture>,
    audio: Option<LiveAudio>,
    shared: Arc<Shared>,
}

enum LiveAudio {
    Window(AudioCapture),
    Mix(Box<AppMix>),
}

/// The apps that play sound during a screen stream, and the capture that
/// mixes the included ones.
struct AppMix {
    capture: MixedAudioCapture,
    apps: Vec<AudioApp>,
    icons: HashMap<PathBuf, Option<String>>,
}

impl AppMix {
    /// Takes the latest list of apps and makes the mix follow the choices.
    fn update(&mut self, apps: Vec<AudioApp>, prefs: &AudioPrefs) {
        for app in &apps {
            self.icons.entry(app.executable.clone()).or_insert_with(|| {
                telai_capture::executable_icon_png(&app.executable)
                    .as_deref()
                    .map(icon_data_url)
            });
        }
        self.apps = apps;
        self.sync(prefs);
    }

    fn sync(&mut self, prefs: &AudioPrefs) {
        let included: Vec<(u32, f32)> = self
            .apps
            .iter()
            .filter(|app| prefs.is_included(&app.name))
            .map(|app| (app.process_id, prefs.volume(&app.name)))
            .collect();

        self.capture.sync(&included);
    }

    fn views(&self, prefs: &AudioPrefs) -> Vec<AudioAppView> {
        let levels: HashMap<u32, f32> = self.capture.take_levels().into_iter().collect();

        self.apps
            .iter()
            .map(|app| {
                let included = prefs.is_included(&app.name);
                AudioAppView {
                    process_id: app.process_id,
                    name: app.name.clone(),
                    icon: self.icons.get(&app.executable).cloned().flatten(),
                    included,
                    volume: prefs.volume(&app.name),
                    level: if included {
                        levels.get(&app.process_id).copied().unwrap_or(0.0)
                    } else {
                        0.0
                    },
                }
            })
            .collect()
    }
}

/// State written by the capture threads and read by the monitor.
struct Shared {
    source: SourceView,
    /// The window's process; 0 for a monitor.
    process_id: u32,
    started_at: u64,
    /// Bit pattern of the peak level, as `f32`. Non-negative floats order the
    /// same way as their bits, so `fetch_max` works on them.
    level: AtomicU32,
    window_closed: AtomicBool,
    stopped: AtomicBool,
}

impl LiveSession {
    fn snapshot(
        &self,
        viewers: usize,
        browser_audible: bool,
        minimized: bool,
        prefs: &AudioPrefs,
    ) -> LiveSnapshot {
        let level = f32::from_bits(self.shared.level.swap(0, Ordering::Relaxed));
        let audio_apps = match &self.audio {
            Some(LiveAudio::Mix(mix)) => Some(mix.views(prefs)),
            Some(LiveAudio::Window(_)) | None => None,
        };

        LiveSnapshot {
            source: self.shared.source.clone(),
            started_at: self.shared.started_at,
            viewers,
            browser_audible,
            minimized,
            level,
            audio_apps,
        }
    }

    fn is_finished(&self) -> bool {
        self.video.as_ref().is_none_or(VideoCapture::is_finished)
    }

    fn mix_mut(&mut self) -> Option<&mut AppMix> {
        match &mut self.audio {
            Some(LiveAudio::Mix(mix)) => Some(mix),
            Some(LiveAudio::Window(_)) | None => None,
        }
    }
}

impl Drop for LiveSession {
    fn drop(&mut self) {
        self.shared.stopped.store(true, Ordering::Relaxed);

        match self.audio.take() {
            Some(LiveAudio::Window(audio)) => audio.stop(),
            Some(LiveAudio::Mix(mix)) => mix.capture.stop(),
            None => {}
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
    let prefs = prefs(app);

    lock_live(app)
        .as_ref()
        .map(|session| session.snapshot(0, false, false, &prefs))
}

/// Replaces any running session with a capture of `source`.
pub fn start(app: &AppHandle, source: SourceRef) -> Result<LiveSnapshot, CommandError> {
    stop(app);

    let session = match source {
        SourceRef::Window { handle } => {
            let window = telai_capture::list_windows()
                .map_err(CommandError::video)?
                .into_iter()
                .find(|window| window.handle == handle)
                .ok_or(CommandError::WindowGone)?;

            open_window_session(app, &window)?
        }
        SourceRef::Monitor { handle } => {
            let monitor = telai_capture::list_monitors()
                .map_err(CommandError::video)?
                .into_iter()
                .find(|monitor| monitor.handle == handle)
                .ok_or(CommandError::WindowGone)?;

            open_monitor_session(app, &monitor)?
        }
    };
    let snapshot = session.snapshot(0, false, false, &prefs(app));
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

/// Saves whether a screen stream carries the app `name`, and how loud, and
/// applies it to the running stream at once.
pub fn set_audio_app(app: &AppHandle, name: &str, included: bool, volume: f32) {
    let state = app.state::<AppState>();
    let prefs = {
        let mut prefs = state
            .audio_prefs
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        prefs.set(name, included, volume);
        prefs.clone()
    };

    if let Some(path) = &state.audio_prefs_path
        && let Err(error) = prefs.save(path)
    {
        tracing::warn!(%error, "the audio choices could not be saved");
    }

    if let Some(mix) = lock_live(app).as_mut().and_then(LiveSession::mix_mut) {
        mix.sync(&prefs);
    }
}

fn open_window_session(app: &AppHandle, window: &WindowInfo) -> Result<LiveSession, CommandError> {
    let shared = new_shared(
        SourceView {
            kind: SourceKind::Window,
            handle: window.handle,
            title: window.title.clone(),
            process_name: window.process_name.clone(),
            icon: window.icon_png.as_deref().map(icon_data_url),
        },
        window.process_id,
    );

    let audio = AudioCapture::start(window.process_id, publisher(app, &shared))
        .map_err(CommandError::audio)?;
    let video = start_video(app, CaptureTarget::Window(window.handle), &shared)?;

    Ok(new_session(app, video, LiveAudio::Window(audio), shared))
}

fn open_monitor_session(
    app: &AppHandle,
    monitor: &MonitorInfo,
) -> Result<LiveSession, CommandError> {
    let title = monitor_title(
        &monitor.name,
        monitor.number,
        monitor.width,
        monitor.height,
        text().screen,
    );
    let shared = new_shared(
        SourceView {
            kind: SourceKind::Monitor,
            handle: monitor.handle,
            title,
            process_name: String::new(),
            icon: None,
        },
        0,
    );

    let capture = MixedAudioCapture::start(publisher(app, &shared)).map_err(CommandError::audio)?;
    let mut mix = AppMix {
        capture,
        apps: Vec::new(),
        icons: HashMap::new(),
    };
    match telai_capture::audio_apps() {
        Ok(apps) => mix.update(apps, &prefs(app)),
        Err(error) => tracing::warn!(%error, "the apps playing sound could not be listed"),
    }

    let video = start_video(app, CaptureTarget::Monitor(monitor.handle), &shared)?;

    Ok(new_session(
        app,
        video,
        LiveAudio::Mix(Box::new(mix)),
        shared,
    ))
}

/// "DELL U2720Q (3840×2160)", or "Screen 2 (1920×1080)" for a display that
/// reports no name.
fn monitor_title(name: &str, number: usize, width: u32, height: u32, screen: &str) -> String {
    let name = name.trim();
    if name.is_empty() {
        format!("{screen} {number} ({width}×{height})")
    } else {
        format!("{name} ({width}×{height})")
    }
}

fn new_shared(source: SourceView, process_id: u32) -> Arc<Shared> {
    let started_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        });

    Arc::new(Shared {
        source,
        process_id,
        started_at,
        level: AtomicU32::new(0),
        window_closed: AtomicBool::new(false),
        stopped: AtomicBool::new(false),
    })
}

/// Sends captured samples to the viewers and keeps the session's peak level.
fn publisher(app: &AppHandle, shared: &Arc<Shared>) -> impl FnMut(&[f32]) + Send + 'static {
    let hub = app.state::<AppState>().hub.clone();
    let shared = Arc::clone(shared);

    move |samples| {
        hub.publish_audio(samples);

        let peak = samples
            .iter()
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
        shared
            .level
            .fetch_max(peak.min(1.0).to_bits(), Ordering::Relaxed);
    }
}

fn start_video(
    app: &AppHandle,
    target: CaptureTarget,
    shared: &Arc<Shared>,
) -> Result<VideoCapture, CommandError> {
    let sink = EncoderSink {
        pool: EncoderPool::new(app.state::<AppState>().hub.clone()),
        shared: Arc::clone(shared),
    };

    VideoCapture::start(target, sink).map_err(CommandError::video)
}

fn new_session(
    app: &AppHandle,
    video: VideoCapture,
    audio: LiveAudio,
    shared: Arc<Shared>,
) -> LiveSession {
    app.state::<AppState>().hub.set_status(Status {
        live: true,
        title: shared.source.title.clone(),
    });

    LiveSession {
        id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        video: Some(video),
        audio: Some(audio),
        shared,
    }
}

/// Reports the session to the window until it stops, and ends it when the
/// captured window or monitor goes away.
fn monitor(app: &AppHandle, id: u64, shared: &Shared) {
    let state = app.state::<AppState>();
    let is_window = shared.source.kind == SourceKind::Window;
    let mut tick = 0;
    let mut minimized = false;
    let mut browser = BrowserAudio::default();

    loop {
        thread::sleep(TICK);
        if shared.stopped.load(Ordering::Relaxed) {
            return;
        }

        let viewers = state.hub.viewers().borrow().clone();
        let mut audio_apps = None;
        if tick % SLOW_CHECK_TICKS == 0 {
            if is_window {
                minimized = telai_capture::is_minimized(shared.source.handle);
            } else {
                audio_apps = list_audio_apps();
            }
            browser.update(&viewers, state.port, shared.process_id);
        }
        tick += 1;

        let prefs = prefs(app);
        let mut live = lock_live(app);
        let Some(session) = live.as_mut().filter(|session| session.id == id) else {
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

        if let (Some(apps), Some(mix)) = (audio_apps, session.mix_mut()) {
            mix.update(apps, &prefs);
        }

        let snapshot = session.snapshot(viewers.len(), browser.audible(), minimized, &prefs);
        drop(live);
        let _ = app.emit(LIVE_EVENT, snapshot);
    }
}

fn list_audio_apps() -> Option<Vec<AudioApp>> {
    telai_capture::audio_apps()
        .inspect_err(|error| tracing::warn!(%error, "the apps playing sound could not be listed"))
        .ok()
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

/// A copy of the saved audio choices, so that no lock is held while the live
/// session is locked.
fn prefs(app: &AppHandle) -> AudioPrefs {
    app.state::<AppState>()
        .audio_prefs
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_monitor_is_titled_by_its_name_and_size() {
        assert_eq!(
            monitor_title("DELL U2720Q", 1, 3840, 2160, "Tela"),
            "DELL U2720Q (3840×2160)"
        );
    }

    #[test]
    fn a_monitor_without_a_name_is_titled_by_its_number() {
        assert_eq!(
            monitor_title("", 2, 1920, 1080, "Tela"),
            "Tela 2 (1920×1080)"
        );
    }
}
