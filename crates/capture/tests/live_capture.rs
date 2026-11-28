//! Captures a real window and its sound. Needs a desktop session and an audio
//! output device, so it runs only on request: `cargo test -- --ignored`.

#![cfg(windows)]

use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use telai_capture::{
    AudioCapture, CaptureTarget, Frame, MixedAudioCapture, VideoCapture, VideoSink, WindowInfo,
    list_windows,
};

const TITLE: &str = "telai-capture-test";
const MIX_TITLE: &str = "telai-mix-test";
const TIMEOUT: Duration = Duration::from_secs(20);
const SAMPLE_RATE: u32 = 48_000;

#[test]
#[ignore = "needs a desktop session and an audio output device"]
fn captures_the_picture_and_sound_of_a_window() {
    let tone = write_tone(&std::env::temp_dir().join("telai-capture-test.wav"));
    let child = TestWindow::spawn(&tone, TITLE);

    let window = wait_for(|| find_window(TITLE, |window| window.playing_audio))
        .expect("the test window is listed as playing sound");
    assert_eq!(window.process_id, child.0.id());

    let thumbnail = window
        .thumbnail_jpeg
        .as_deref()
        .expect("the window has a thumbnail");
    assert_eq!(&thumbnail[..2], &[0xFF, 0xD8], "the thumbnail is a JPEG");

    let peak = capture_audio_peak(window.process_id);
    assert!(peak > 0.05, "captured audio peak {peak}");

    let (width, height, center) = capture_first_frame(CaptureTarget::Window(window.handle));
    assert!(width >= 320 && height >= 160, "frame of {width}x{height}");
    assert_eq!(center, [0, 0, 255], "the center pixel is red, in BGR order");
}

#[test]
#[ignore = "needs a desktop session"]
fn lists_and_captures_a_monitor() {
    let monitors = telai_capture::list_monitors().expect("monitors are listed");
    let first = monitors.first().expect("at least one monitor");
    assert_eq!(first.number, 1);
    assert!(first.width > 0 && first.height > 0);
    assert_eq!(monitors.iter().filter(|monitor| monitor.primary).count(), 1);

    let (width, height, _) = capture_first_frame(CaptureTarget::Monitor(first.handle));
    assert_eq!((width, height), (first.width, first.height));
}

#[test]
#[ignore = "needs a desktop session and an audio output device"]
fn mixes_only_the_included_apps() {
    let tone = write_tone(&std::env::temp_dir().join("telai-mix-test.wav"));
    let child = TestWindow::spawn(&tone, MIX_TITLE);
    let window = wait_for(|| find_window(MIX_TITLE, |window| window.playing_audio))
        .expect("the test tone plays");

    let included = mixed_peak(&[(window.process_id, 1.0)]);
    assert!(included > 0.05, "included app peak {included}");

    let excluded = mixed_peak(&[]);
    assert!(excluded < 0.001, "excluded app peak {excluded}");

    drop(child);
}

/// Highest level the mix publishes within three seconds of including `apps`.
fn mixed_peak(apps: &[(u32, f32)]) -> f32 {
    let peak = Arc::new(Mutex::new(0.0_f32));
    let published = Arc::clone(&peak);
    let mut mix = MixedAudioCapture::start(move |samples| {
        let chunk = samples
            .iter()
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
        let mut peak = published.lock().unwrap();
        *peak = peak.max(chunk);
    })
    .expect("the mixer starts");

    mix.sync(apps);
    std::thread::sleep(Duration::from_secs(3));
    mix.stop();

    *peak.lock().unwrap()
}

/// Highest sample level within a few seconds of capture.
fn capture_audio_peak(process_id: u32) -> f32 {
    let (sender, receiver) = mpsc::channel();
    let audio = AudioCapture::start(process_id, move |samples| {
        let peak = samples
            .iter()
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
        let _ = sender.send(peak);
    })
    .expect("audio capture starts");

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut peak = 0.0_f32;
    while Instant::now() < deadline && peak <= 0.05 {
        if let Ok(chunk_peak) = receiver.recv_timeout(Duration::from_millis(200)) {
            peak = peak.max(chunk_peak);
        }
    }
    audio.stop();

    peak
}

fn capture_first_frame(target: CaptureTarget) -> (u32, u32, [u8; 3]) {
    let (sender, receiver) = mpsc::channel();
    let video = VideoCapture::start(target, ChannelSink(sender)).expect("window capture starts");

    let frame = receiver.recv_timeout(TIMEOUT).expect("a frame arrives");
    video.stop();

    frame
}

struct ChannelSink(mpsc::Sender<(u32, u32, [u8; 3])>);

impl VideoSink for ChannelSink {
    fn frame(&mut self, frame: &Frame<'_>) {
        let mut bgra = Vec::new();
        frame.copy_to(&mut bgra);

        let center =
            (frame.height as usize / 2 * frame.width as usize + frame.width as usize / 2) * 4;
        let _ = self.0.send((
            frame.width,
            frame.height,
            [bgra[center], bgra[center + 1], bgra[center + 2]],
        ));
    }

    fn closed(&mut self) {}
}

/// A red form in its own PowerShell process, looping a tone.
struct TestWindow(Child);

impl TestWindow {
    fn spawn(tone: &Path, title: &str) -> Self {
        let script = format!(
            "Add-Type -AssemblyName System.Windows.Forms; \
             $player = New-Object System.Media.SoundPlayer '{}'; \
             $player.PlayLooping(); \
             $form = New-Object System.Windows.Forms.Form; \
             $form.Text = '{title}'; \
             $form.BackColor = [System.Drawing.Color]::Red; \
             $form.Width = 640; \
             $form.Height = 360; \
             [System.Windows.Forms.Application]::Run($form)",
            tone.display(),
        );

        let child = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .spawn()
            .expect("PowerShell starts");

        Self(child)
    }
}

impl Drop for TestWindow {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn find_window(title: &str, condition: impl Fn(&WindowInfo) -> bool) -> Option<WindowInfo> {
    list_windows()
        .ok()?
        .into_iter()
        .find(|window| window.title == title && condition(window))
}

fn wait_for<T>(probe: impl Fn() -> Option<T>) -> Option<T> {
    let deadline = Instant::now() + TIMEOUT;
    while Instant::now() < deadline {
        if let Some(value) = probe() {
            return Some(value);
        }
        std::thread::sleep(Duration::from_millis(250));
    }

    None
}

/// Writes one second of a 440 Hz tone at half scale as a 16-bit mono WAV file.
fn write_tone(path: &Path) -> PathBuf {
    let samples: Vec<i16> = (0..SAMPLE_RATE)
        .map(|index| {
            let phase = f64::from(index) * 440.0 / f64::from(SAMPLE_RATE);
            let value = (phase * std::f64::consts::TAU).sin() * f64::from(i16::MAX) / 2.0;
            #[allow(
                clippy::cast_possible_truncation,
                reason = "the value is within i16 range"
            )]
            let sample = value as i16;
            sample
        })
        .collect();

    let data_len = u32::try_from(samples.len() * 2).expect("one second fits u32");
    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    wav.extend_from_slice(&2_u16.to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        wav.extend_from_slice(&sample.to_le_bytes());
    }

    std::fs::write(path, wav).expect("the tone file is written");

    path.to_path_buf()
}
