use std::{collections::VecDeque, time::Duration};

use instant::Instant;

/// How much audio one level covers: short enough for the bars to follow
/// syllables.
pub(super) const LEVEL_INTERVAL: Duration = Duration::from_millis(25);

/// How many recent levels are kept: enough to fill a wide waveform (about six
/// seconds of audio).
pub(super) const LEVEL_HISTORY: usize = 256;

/// Raw levels below this are background noise and read as silence, so a quiet
/// room shows a calm baseline instead of flickering bars.
const NOISE_FLOOR: f32 = 0.06;

/// How far a level moves toward a louder reading per step: almost at once, so
/// peaks pop.
const ATTACK: f32 = 0.85;

/// How far a level moves toward a quieter reading per step: slowly, so bars
/// fall back smoothly.
const RELEASE: f32 = 0.3;

/// Turns a stream of PCM into smoothed input levels, one per
/// [`LEVEL_INTERVAL`] of audio, carrying partial intervals across pushes.
pub(super) struct LevelMeter {
    /// Samples per level: [`LEVEL_INTERVAL`] at the stream's rate and channels.
    window: usize,
    sum: f64,
    count: usize,
    smoothed: f32,
    levels: VecDeque<f32>,
    last_level_at: Option<Instant>,
}

impl LevelMeter {
    pub(super) fn new() -> Self {
        Self {
            window: window_for(16_000, 1),
            sum: 0.,
            count: 0,
            smoothed: 0.,
            levels: VecDeque::with_capacity(LEVEL_HISTORY),
            last_level_at: None,
        }
    }

    /// Clear the history and measure a stream of `sample_rate` × `channels`.
    pub(super) fn reset(&mut self, sample_rate: u32, channels: u16) {
        *self = Self {
            window: window_for(sample_rate, channels),
            ..Self::new()
        };
    }

    /// Measure `samples`; returns whether a new level was recorded.
    pub(super) fn push(&mut self, samples: &[i16]) -> bool {
        let mut recorded = false;
        for &sample in samples {
            let sample = sample as f64 / i16::MAX as f64;
            self.sum += sample * sample;
            self.count += 1;
            if self.count == self.window {
                let raw = gate(level_of_rms((self.sum / self.count as f64).sqrt()));
                self.smoothed = smooth(self.smoothed, raw);
                if self.levels.len() == LEVEL_HISTORY {
                    self.levels.pop_front();
                }
                self.levels.push_back(self.smoothed);
                self.sum = 0.;
                self.count = 0;
                recorded = true;
            }
        }
        if recorded {
            self.last_level_at = Some(Instant::now());
        }
        recorded
    }

    pub(super) fn levels(&self) -> impl ExactSizeIterator<Item = f32> + '_ {
        self.levels.iter().copied()
    }

    /// When the newest level was recorded, to scroll smoothly between levels.
    pub(super) fn last_level_at(&self) -> Option<Instant> {
        self.last_level_at
    }
}

fn window_for(sample_rate: u32, channels: u16) -> usize {
    let per_second = sample_rate as u128 * channels.max(1) as u128;
    ((per_second * LEVEL_INTERVAL.as_millis() / 1_000) as usize).max(1)
}

/// The loudness of an RMS amplitude in `0.0..=1.0`, mapping -50 dBFS..0 dBFS
/// linearly so that normal speech fills most of the range.
pub(super) fn level_of_rms(rms: f64) -> f32 {
    if rms <= 0. {
        return 0.;
    }
    let db = 20. * rms.log10();
    ((db + 50.) / 50.).clamp(0., 1.) as f32
}

fn gate(level: f32) -> f32 {
    if level < NOISE_FLOOR { 0. } else { level }
}

/// One step of fast-attack, slow-release smoothing from `previous` toward `raw`.
pub(super) fn smooth(previous: f32, raw: f32) -> f32 {
    let rate = if raw > previous { ATTACK } else { RELEASE };
    previous + (raw - previous) * rate
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(amplitude: i16, len: usize) -> Vec<i16> {
        (0..len)
            .map(|ix| if ix % 2 == 0 { amplitude } else { -amplitude })
            .collect()
    }

    #[test]
    fn one_level_per_interval_across_pushes() {
        let mut meter = LevelMeter::new();
        meter.reset(16_000, 1);
        // 25 ms at 16 kHz is 400 samples; feed 1 000 samples in uneven pushes.
        assert!(!meter.push(&tone(1_000, 150)));
        assert!(meter.push(&tone(1_000, 300)));
        assert!(meter.push(&tone(1_000, 550)));
        assert_eq!(meter.levels().len(), 2);
        assert!(meter.last_level_at().is_some());
    }

    #[test]
    fn the_window_follows_the_stream_format() {
        assert_eq!(window_for(16_000, 1), 400);
        assert_eq!(window_for(48_000, 2), 2_400);
    }

    #[test]
    fn peaks_rise_fast_and_fall_slowly() {
        let up = smooth(0., 1.);
        assert!(up >= 0.85);
        let down = smooth(1., 0.);
        assert!(down >= 0.65, "{down}");
        assert!(smooth(down, 0.) < down);
    }

    #[test]
    fn background_noise_reads_as_silence() {
        let mut meter = LevelMeter::new();
        meter.reset(16_000, 1);
        // -60 dBFS: below the -50 dB floor of the scale.
        meter.push(&tone(32, 400));
        assert_eq!(meter.levels().next(), Some(0.));
    }

    #[test]
    fn level_maps_decibels_to_the_unit_range() {
        assert_eq!(level_of_rms(0.), 0.);
        assert_eq!(level_of_rms(1.), 1.);
        // -20 dBFS sits at 0.6 on the -50..0 dB scale.
        assert!((level_of_rms(0.1) - 0.6).abs() < 0.01);
    }

    #[test]
    fn history_is_bounded() {
        let mut meter = LevelMeter::new();
        meter.reset(16_000, 1);
        meter.push(&tone(8_000, 400 * (LEVEL_HISTORY + 10)));
        assert_eq!(meter.levels().len(), LEVEL_HISTORY);
    }
}
