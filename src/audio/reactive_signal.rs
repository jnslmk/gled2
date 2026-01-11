use crate::audio::state::{FREQ_BINS, MAX_BIN_FREQ};
use std::f32::consts::TAU;
use crate::audio::ADSR_SAMPLE_INTERVAL_MS;

#[derive(Debug, Clone)]
pub struct AdsrParams {
    center_bin: usize,
    bin_radius: usize,
    pub attack_duration: f32,  // in seconds
    pub decay_duration: f32,   // in seconds
    pub sustain_level: f32,    // 0.0 to 1.0
    pub release_duration: f32, // in seconds
    /// Threshold to consider the continuous input "ON"
    pub trigger_happiness: f32,
    pub gate_threshold: f32,
}
impl AdsrParams {
    pub fn new(
        f_center: f32,
        f_radius: f32,
        attack_duration: f32,
        decay_duration: f32,
        sustain_level: f32,
        release_duration: f32,
        trigger_happiness: f32,
        gate_threshold: f32,
    ) -> Self {
        let mut ret = Self {
            center_bin: 0,
            bin_radius: 0,
            attack_duration,
            decay_duration,
            sustain_level,
            release_duration,
            trigger_happiness,
            gate_threshold,
        };
        ret.set_filtertune(f_center, f_radius);
        ret
    }
    pub fn set_filtertune(&mut self, f_center: f32, f_radius: f32) {
        let f_per_bin = MAX_BIN_FREQ / FREQ_BINS as f32;
        let center_bin = ((f_center / f_per_bin).round() as usize).min(FREQ_BINS - 1);
        let bin_radius = (f_radius / f_per_bin).round() as usize;
        self.center_bin = center_bin;
        self.bin_radius = bin_radius;
    }
}

impl Default for AdsrParams {
    fn default() -> Self {
        AdsrParams::new(440.,
                        100.,
                        0.1,
                        0.2,
                        0.5,
                        1.,
                        10.,
                        0.5
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AdsrPhase {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

impl Default for AdsrPhase {
    fn default() -> Self {
        AdsrPhase::Idle
    }
}

pub struct ReactiveSignal {
    pub phase: AdsrPhase,
    pub params: AdsrParams,
    pub current_spectrum: Vec<f32>,
    pub impulse: f32,
    pub delta_time: f32,
    pub slow_ema: f32,
    pub fast_ema: f32,
    pub current_level: f32,
}

impl Default for ReactiveSignal {
    fn default() -> Self {
        ReactiveSignal::new(AdsrParams::default(), ADSR_SAMPLE_INTERVAL_MS as f32 / 1000.)
    }
}

impl ReactiveSignal {
    pub fn new(params: AdsrParams, delta_time: f32) -> Self {
        Self {
            current_spectrum: vec![0f32; FREQ_BINS],
            delta_time,
            slow_ema: 0.,
            fast_ema: 0.,
            phase: AdsrPhase::Idle,
            params,
            current_level: 0.0,
            impulse: 0.,
        }
    }

    pub fn tick(&mut self, input: &Vec<f32>) {
        self.impulse = self.tick_lowpass(input).log2().clamp(0.0, 1.0);
        self.tick_adsr(self.impulse);
    }

    /// Calculate the envelope amplitude at the current timepoint.
    ///
    /// # Arguments
    /// * `input` - A continuous float control signal (Gate).
    ///             > 0.5 triggers Attack/Sustain.
    ///             <= 0.5 triggers Release.
    pub fn tick_adsr(&mut self, input: f32) -> f32 {
        let is_gate_open = input > self.params.gate_threshold;

        // 1. Handle State Transitions
        match self.phase {
            AdsrPhase::Idle => {
                if is_gate_open {
                    self.phase = AdsrPhase::Attack;
                }
            }
            AdsrPhase::Attack => {
                if !is_gate_open {
                    self.phase = AdsrPhase::Release;
                } else if self.current_level >= 1.0 {
                    self.phase = AdsrPhase::Decay;
                }
            }
            AdsrPhase::Decay => {
                if !is_gate_open {
                    self.phase = AdsrPhase::Release;
                } else if self.current_level <= self.params.sustain_level {
                    self.phase = AdsrPhase::Sustain;
                }
            }
            AdsrPhase::Sustain => {
                if !is_gate_open {
                    self.phase = AdsrPhase::Release;
                }
                // In Sustain, we just hold the level (handled in calculation step)
            }
            AdsrPhase::Release => {
                if is_gate_open {
                    // Re-trigger: Standard ADSR behavior is to Attack from current level
                    self.phase = AdsrPhase::Attack;
                } else if self.current_level <= 0.0 {
                    self.phase = AdsrPhase::Idle;
                }
            }
        }

        // 2. Calculate Amplitude Changes (Linear Logic)
        match self.phase {
            AdsrPhase::Idle => {
                self.current_level = 0.0;
            }
            AdsrPhase::Attack => {
                // Rate = 1.0 / duration
                let rate = 1.0 / self.params.attack_duration.max(0.001);
                self.current_level += rate * self.delta_time;
                self.current_level = self.current_level.min(1.0);
            }
            AdsrPhase::Decay => {
                // Rate = distance to sustain / duration
                let dist = 1.0 - self.params.sustain_level;
                let rate = dist / self.params.decay_duration.max(0.001);
                self.current_level -= rate * self.delta_time;
                self.current_level = self.current_level.max(self.params.sustain_level);
            }
            AdsrPhase::Sustain => {
                // Constant level
                self.current_level = self.params.sustain_level;
            }
            AdsrPhase::Release => {
                // Rate = 1.0 / duration (standard is to release from max)
                // or proportional to current level (exponential-ish)
                // Here we use linear release from MAX for consistency
                let rate = 1.0 / self.params.release_duration.max(0.001);
                self.current_level -= rate * self.delta_time;
                self.current_level = self.current_level.max(0.0);
            }
        }

        // 3. Final Safety Clamp
        self.current_level = self.current_level.clamp(0.0, 1.0);

        self.current_level
    }

    #[inline(always)]
    pub fn tick_lowpass(&mut self, bins: &Vec<f32>) -> f32 {
        let amplitude = bins[(self.params.center_bin.saturating_sub(self.params.bin_radius)).clamp(0, FREQ_BINS - 1)
            ..self.params.center_bin + self.params.bin_radius.clamp(0, FREQ_BINS - 1)]
            .iter()
            .sum::<f32>()
            / (2. * self.params.bin_radius as f32 + 1.) + 1.;

        // calculate an exponential moving average to prevent aliasing
        let mut alpha = 1.0 - (-self.delta_time / TAU).exp(); // Sample-rate-aware alpha
        alpha *= 10.;
        self.slow_ema = alpha * amplitude + (1.0 - alpha) * self.slow_ema;
        alpha *= self.params.trigger_happiness;
        self.fast_ema = alpha * amplitude + (1.0 - alpha) * self.fast_ema;
        self.fast_ema / self.slow_ema
    }
}
