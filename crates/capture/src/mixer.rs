use std::collections::{BTreeMap, VecDeque};

const CHANNELS: usize = 2;

/// 40 ms at 48 kHz: how much a source buffers before it is heard, so that
/// capture packets arriving a little late do not leave gaps.
pub(crate) const TARGET_FRAMES: usize = 1_920;

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

            let available = source.samples.len().min(wanted);
            for (mixed, sample) in out.iter_mut().zip(source.samples.drain(..available)) {
                let sample = sample * source.gain;
                source.peak = source.peak.max(sample.abs());
                *mixed += sample;
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
