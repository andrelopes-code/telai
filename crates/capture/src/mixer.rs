use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use crate::{AudioCapture, Error};

const CHANNELS: usize = 2;

const SAMPLE_RATE: u64 = 48_000;

/// 10 ms at 48 kHz, the size of each published chunk.
const CHUNK_FRAMES: usize = 480;

/// After a stall longer than this, the mixer skips ahead instead of
/// publishing the whole backlog at once.
const MAX_CATCH_UP_FRAMES: u64 = 4_800;

/// How long the mixer thread sleeps between checks of the clock.
const TICK: Duration = Duration::from_millis(5);

/// 40 ms at 48 kHz: how much a source buffers before it is heard, so that
/// capture packets arriving a little late do not leave gaps.
pub(crate) const TARGET_FRAMES: usize = 1_920;

/// 20 ms at 48 kHz: how far a source's buffer may stray from
/// [`TARGET_FRAMES`] before its clock drift is corrected. Wider than the
/// jitter of 10 ms capture packets.
const SLACK_FRAMES: usize = 960;

/// 100 ms at 48 kHz. A source that piles up more than this drops its oldest
/// samples down to [`TARGET_FRAMES`], so latency never grows.
pub(crate) const MAX_FRAMES: usize = 4_800;

/// Sums the interleaved stereo samples of several sources, each with its own
/// gain, into one stream.
#[derive(Default)]
pub(crate) struct Mixer {
    sources: BTreeMap<u32, Source>,
}

struct Source {
    samples: VecDeque<f32>,
    gain: f32,
    /// Whether the source buffered [`TARGET_FRAMES`] since it started or
    /// last ran dry.
    primed: bool,
    /// Highest absolute sample after gain since the last report.
    peak: f32,
}

impl Default for Source {
    fn default() -> Self {
        Self {
            samples: VecDeque::new(),
            gain: 1.0,
            primed: false,
            peak: 0.0,
        }
    }
}

impl Mixer {
    pub(crate) fn push(&mut self, id: u32, samples: &[f32]) {
        let source = self.sources.entry(id).or_default();
        source.samples.extend(samples);

        if source.samples.len() > MAX_FRAMES * CHANNELS {
            let excess = source.samples.len() - TARGET_FRAMES * CHANNELS;
            source.samples.drain(..excess);
        }
    }

    pub(crate) fn set_gain(&mut self, id: u32, gain: f32) {
        let gain = if gain.is_nan() {
            0.0
        } else {
            gain.clamp(0.0, 1.0)
        };
        self.sources.entry(id).or_default().gain = gain;
    }

    pub(crate) fn remove(&mut self, id: u32) {
        self.sources.remove(&id);
    }

    /// Replaces `out` with the next `frames` frames of the mix. Sources that
    /// are not primed, or run dry, contribute silence.
    pub(crate) fn mix(&mut self, frames: usize, out: &mut Vec<f32>) {
        let wanted = frames * CHANNELS;
        out.clear();
        out.resize(wanted, 0.0);

        for source in self.sources.values_mut() {
            if !source.primed {
                if source.samples.len() < TARGET_FRAMES * CHANNELS {
                    continue;
                }
                source.primed = true;
            }

            let buffered = source.samples.len() / CHANNELS;
            let drift = drift_correction(buffered, frames);
            let available = source.samples.len().min(wanted);
            let read = if drift == Drift::Behind {
                available - CHANNELS
            } else {
                available
            };

            let mut last = [0.0; CHANNELS];
            for (index, (mixed, sample)) in
                out.iter_mut().zip(source.samples.drain(..read)).enumerate()
            {
                let sample = sample * source.gain;
                source.peak = source.peak.max(sample.abs());
                *mixed += sample;
                last[index % CHANNELS] = sample;
            }

            match drift {
                Drift::Behind => {
                    for (mixed, sample) in out[read..read + CHANNELS].iter_mut().zip(last) {
                        *mixed += sample;
                    }
                }
                Drift::Ahead => {
                    source.samples.drain(..CHANNELS);
                }
                Drift::None => {}
            }

            if available < wanted {
                source.primed = false;
            }
        }

        for sample in out.iter_mut() {
            *sample = sample.clamp(-1.0, 1.0);
        }
    }

    /// Each source's peak since the previous call, by id.
    pub(crate) fn take_peaks(&mut self) -> Vec<(u32, f32)> {
        self.sources
            .iter_mut()
            .map(|(id, source)| (*id, std::mem::take(&mut source.peak)))
            .collect()
    }
}

/// Which way a source's buffer drifts from [`TARGET_FRAMES`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Drift {
    None,
    /// The app's audio clock runs faster than the mixer's: one frame is
    /// skipped.
    Ahead,
    /// The app's audio clock runs slower: one frame is played twice.
    Behind,
}

/// The audio device and the mixer keep time with different clocks, which
/// differ by tens of parts per million. Skipping or repeating one frame when
/// the buffer leaves the band around the target keeps latency steady with no
/// audible gap, where waiting for an underrun or a trim would drop 40 ms or
/// more every few minutes.
fn drift_correction(buffered: usize, frames: usize) -> Drift {
    if buffered > TARGET_FRAMES + SLACK_FRAMES {
        Drift::Ahead
    } else if buffered < TARGET_FRAMES - SLACK_FRAMES && buffered >= frames && frames > 1 {
        Drift::Behind
    } else {
        Drift::None
    }
}

/// The sound of several apps, each captured with its child processes and
/// mixed into one interleaved stereo `f32` stream at 48 kHz.
///
/// The mix is published in real time even when no app plays, so a listener
/// receives silence instead of a gap.
pub struct MixedAudioCapture {
    mixer: Arc<Mutex<Mixer>>,
    captures: BTreeMap<u32, AudioCapture>,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl MixedAudioCapture {
    /// Starts publishing the mix, silent until [`Self::sync`] adds apps.
    pub fn start(on_samples: impl FnMut(&[f32]) + Send + 'static) -> Result<Self, Error> {
        let mixer = Arc::new(Mutex::new(Mixer::default()));
        let stop = Arc::new(AtomicBool::new(false));

        let thread = thread::Builder::new().name("audio-mixer".into()).spawn({
            let mixer = Arc::clone(&mixer);
            let stop = Arc::clone(&stop);
            move || publish_until_stopped(&mixer, &stop, on_samples)
        })?;

        Ok(Self {
            mixer,
            captures: BTreeMap::new(),
            stop,
            thread: Some(thread),
        })
    }

    /// Makes the mix carry exactly `apps`, as (process id, gain) pairs:
    /// starts capturing new ones, stops the ones left out and updates gains.
    /// An app whose capture cannot start is tried again on the next call.
    pub fn sync(&mut self, apps: &[(u32, f32)]) {
        let left_out: Vec<u32> = self
            .captures
            .keys()
            .copied()
            .filter(|id| !apps.iter().any(|(app, _)| app == id))
            .collect();
        for id in left_out {
            if let Some(capture) = self.captures.remove(&id) {
                capture.stop();
            }
            lock(&self.mixer).remove(id);
        }

        for &(id, gain) in apps {
            lock(&self.mixer).set_gain(id, gain);
            if self.captures.contains_key(&id) {
                continue;
            }

            let mixer = Arc::clone(&self.mixer);
            match AudioCapture::start(id, move |samples| lock(&mixer).push(id, samples)) {
                Ok(capture) => {
                    self.captures.insert(id, capture);
                }
                Err(error) => {
                    tracing::debug!(%error, process_id = id, "app audio capture did not start");
                    lock(&self.mixer).remove(id);
                }
            }
        }
    }

    /// Each app's peak level, after gain, since the previous call.
    #[must_use]
    pub fn take_levels(&self) -> Vec<(u32, f32)> {
        lock(&self.mixer).take_peaks()
    }

    pub fn stop(mut self) {
        self.stop_all();
    }

    fn stop_all(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }

        for (_, capture) in std::mem::take(&mut self.captures) {
            capture.stop();
        }
    }
}

impl Drop for MixedAudioCapture {
    fn drop(&mut self) {
        self.stop_all();
    }
}

fn publish_until_stopped(
    mixer: &Mutex<Mixer>,
    stop: &AtomicBool,
    mut on_samples: impl FnMut(&[f32]),
) {
    let started = Instant::now();
    let mut sent: u64 = 0;
    let mut chunk = Vec::with_capacity(CHUNK_FRAMES * CHANNELS);

    while !stop.load(Ordering::Relaxed) {
        let due = frames_due(started.elapsed());
        if due.saturating_sub(sent) > MAX_CATCH_UP_FRAMES {
            sent = due - CHUNK_FRAMES as u64;
        }

        while sent + CHUNK_FRAMES as u64 <= due {
            lock(mixer).mix(CHUNK_FRAMES, &mut chunk);
            on_samples(&chunk);
            sent += CHUNK_FRAMES as u64;
        }

        thread::sleep(TICK);
    }
}

/// Frames that a 48 kHz stream has played after `elapsed`.
fn frames_due(elapsed: Duration) -> u64 {
    u64::try_from(elapsed.as_micros() * u128::from(SAMPLE_RATE) / 1_000_000).unwrap_or(u64::MAX)
}

fn lock(mixer: &Mutex<Mixer>) -> MutexGuard<'_, Mixer> {
    mixer.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frames(value: f32, count: usize) -> Vec<f32> {
        vec![value; count * 2]
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn silence_flows_with_no_sources() {
        let mut mixer = Mixer::default();
        let mut out = vec![9.0];

        mixer.mix(480, &mut out);

        assert_eq!(out.len(), 960);
        assert!(out.iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn primed_sources_are_summed_with_their_gain() {
        let mut mixer = Mixer::default();
        mixer.push(1, &frames(0.25, TARGET_FRAMES));
        mixer.push(2, &frames(0.5, TARGET_FRAMES));
        mixer.set_gain(2, 0.5);
        let mut out = Vec::new();

        mixer.mix(480, &mut out);

        assert_eq!(out.len(), 960);
        assert!(out.iter().all(|sample| close(*sample, 0.5)));
    }

    #[test]
    fn a_source_is_silent_until_primed() {
        let mut mixer = Mixer::default();
        mixer.push(1, &frames(0.5, TARGET_FRAMES - 1));
        let mut out = Vec::new();

        mixer.mix(480, &mut out);

        assert!(out.iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn an_underrun_pads_with_silence_and_waits_to_prime_again() {
        let mut mixer = Mixer::default();
        mixer.push(1, &frames(0.5, TARGET_FRAMES));
        let mut out = Vec::new();

        mixer.mix(TARGET_FRAMES + 10, &mut out);
        assert!(close(out[0], 0.5));
        assert!(out[TARGET_FRAMES * 2..].iter().all(|sample| *sample == 0.0));

        mixer.push(1, &frames(0.5, 10));
        mixer.mix(10, &mut out);
        assert!(
            out.iter().all(|sample| *sample == 0.0),
            "not primed again yet"
        );
    }

    #[test]
    fn the_sum_is_clamped_to_full_scale() {
        let mut mixer = Mixer::default();
        mixer.push(1, &frames(0.9, TARGET_FRAMES));
        mixer.push(2, &frames(-0.9, TARGET_FRAMES));
        mixer.push(3, &frames(0.9, TARGET_FRAMES));
        mixer.push(4, &frames(0.9, TARGET_FRAMES));
        let mut out = Vec::new();

        mixer.mix(10, &mut out);

        assert!(out.iter().all(|sample| close(*sample, 1.0)));
    }

    #[test]
    fn a_removed_source_stops_contributing() {
        let mut mixer = Mixer::default();
        mixer.push(1, &frames(0.5, TARGET_FRAMES));
        mixer.remove(1);
        let mut out = Vec::new();

        mixer.mix(10, &mut out);

        assert!(out.iter().all(|sample| *sample == 0.0));
    }

    fn buffered(mixer: &Mixer, id: u32) -> usize {
        mixer.sources[&id].samples.len() / CHANNELS
    }

    #[test]
    fn a_source_running_ahead_gives_up_one_extra_frame() {
        let mut mixer = Mixer::default();
        mixer.push(1, &frames(0.5, TARGET_FRAMES + SLACK_FRAMES + 100));
        let mut out = Vec::new();

        mixer.mix(480, &mut out);

        assert_eq!(out.len(), 960);
        assert_eq!(
            buffered(&mixer, 1),
            TARGET_FRAMES + SLACK_FRAMES + 100 - 481
        );
    }

    #[test]
    fn a_source_running_behind_repeats_one_frame() {
        let mut mixer = Mixer::default();
        let frames_total = u16::try_from(TARGET_FRAMES).unwrap();
        let ramp: Vec<f32> = (0..frames_total)
            .flat_map(|frame| {
                let value = f32::from(frame) / f32::from(frames_total);
                [value, value]
            })
            .collect();
        mixer.push(1, &ramp);
        let mut out = Vec::new();

        for _ in 0..3 {
            mixer.mix(480, &mut out);
        }
        // 480 frames left: below the band, so one frame is held back.
        mixer.mix(480, &mut out);

        assert_eq!(buffered(&mixer, 1), 1);
        assert!(close(out[956], out[958]), "the last frame repeats");
        assert!(out.iter().all(|sample| *sample > 0.0));
    }

    #[test]
    fn a_source_within_the_band_is_read_as_is() {
        let mut mixer = Mixer::default();
        mixer.push(1, &frames(0.5, TARGET_FRAMES));
        let mut out = Vec::new();

        mixer.mix(480, &mut out);

        assert_eq!(buffered(&mixer, 1), TARGET_FRAMES - 480);
    }

    #[test]
    fn a_backlog_is_trimmed_to_the_target() {
        let mut mixer = Mixer::default();
        mixer.push(1, &frames(0.1, MAX_FRAMES));
        mixer.push(1, &frames(0.2, 10));
        let mut out = Vec::new();

        mixer.mix(TARGET_FRAMES, &mut out);

        assert!(close(out[out.len() - 1], 0.2), "newest samples are kept");
        assert!(close(out[0], 0.1), "only down to the target");
    }

    #[test]
    fn gain_is_kept_between_zero_and_one() {
        let mut mixer = Mixer::default();
        mixer.set_gain(1, 3.0);
        mixer.push(1, &frames(0.5, TARGET_FRAMES));
        let mut out = Vec::new();

        mixer.mix(1, &mut out);

        assert!(close(out[0], 0.5));
    }

    #[test]
    fn peaks_are_reported_after_gain_and_reset() {
        let mut mixer = Mixer::default();
        mixer.push(7, &frames(0.8, TARGET_FRAMES));
        mixer.set_gain(7, 0.5);
        let mut out = Vec::new();

        mixer.mix(10, &mut out);
        let first = mixer.take_peaks();
        let second = mixer.take_peaks();

        assert_eq!(first.len(), 1);
        assert_eq!(first[0].0, 7);
        assert!(close(first[0].1, 0.4));
        assert!(close(second[0].1, 0.0));
    }
}
