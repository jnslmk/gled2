use crate::audio::state::{FREQ_BINS, MAX_BIN_FREQ};
use std::f32::consts::TAU;
use rustfft::num_complex::ComplexFloat;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AdsrPhase {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Debug, Clone)]
pub struct AdsrParams {
    pub attack_duration: f32,  // in seconds
    pub decay_duration: f32,   // in seconds
    pub sustain_level: f32,    // 0.0 to 1.0
    pub release_duration: f32, // in seconds
    /// Threshold to consider the continuous input "ON"
    pub gate_threshold: f32,
}

impl Default for AdsrParams {
    fn default() -> Self {
        AdsrParams {
            attack_duration: 0.1, // 100ms
            decay_duration: 0.2,  // 200ms
            sustain_level: 0.5,   // 50% amplitude
            release_duration: 1., // 500ms
            gate_threshold: 0.5,
        }
    }
}

pub struct Adsr {
    pub params: AdsrParams,
    sample_rate: f32,
    phase: AdsrPhase,
    current_level: f32,
}

impl Adsr {
    pub fn new(sample_rate: f32, params: AdsrParams) -> Self {
        Self {
            params,
            sample_rate,
            phase: AdsrPhase::Idle,
            current_level: 0.0,
        }
    }

    /// Calculate the envelope amplitude at the current timepoint.
    ///
    /// # Arguments
    /// * `input` - A continuous float control signal (Gate).
    ///             > 0.5 triggers Attack/Sustain.
    ///             <= 0.5 triggers Release.
    pub fn tick(&mut self, input: f32) -> f32 {
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
        let dt = 1.0 / self.sample_rate;

        match self.phase {
            AdsrPhase::Idle => {
                self.current_level = 0.0;
            }
            AdsrPhase::Attack => {
                // Rate = 1.0 / duration
                let rate = 1.0 / self.params.attack_duration.max(0.001);
                self.current_level += rate * dt;
                self.current_level = self.current_level.min(1.0);
            }
            AdsrPhase::Decay => {
                // Rate = distance to sustain / duration
                let dist = 1.0 - self.params.sustain_level;
                let rate = dist / self.params.decay_duration.max(0.001);
                self.current_level -= rate * dt;
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
                self.current_level -= rate * dt;
                self.current_level = self.current_level.max(0.0);
            }
        }

        // 3. Final Safety Clamp
        self.current_level = self.current_level.clamp(0.0, 1.0);

        self.current_level
    }

    /// Optional: Update parameters in real-time
    pub fn set_params(&mut self, params: AdsrParams) {
        self.params = params;
    }
}

pub struct LowPass {
    center_bin: usize,
    bin_radius: usize,
    delta_time: f32,
    slow_ema: f32,
    fast_ema: f32,
    pub trigger_happiness: f32,
}

impl LowPass {
    pub fn new(f_center: f32, f_radius: f32, sample_rate: f32) -> Self {
        let f_per_bin = MAX_BIN_FREQ / FREQ_BINS as f32;
        let center_bin = ((f_center / f_per_bin).round() as usize).min(FREQ_BINS - 1);
        let use_bins = (f_radius / f_per_bin).round() as usize;
        LowPass {
            center_bin,
            bin_radius: use_bins,
            delta_time: 10. / sample_rate,
            slow_ema: 0.,
            fast_ema: 0.,
            trigger_happiness: 10.,
        }
    }

    pub fn set_trigger_happyness(&mut self, happyness: f32) {
        self.trigger_happiness = happyness.expf(10.);
    }

    // minimalistic filter over frequency bins
    pub fn tick(&mut self, bins: &Vec<f32>) -> f32 {
        let amplitude = bins[(self.center_bin - self.bin_radius).clamp(0, FREQ_BINS - 1)
            ..self.center_bin + self.bin_radius.clamp(0, FREQ_BINS - 1)]
            .iter()
            .sum::<f32>()
            * 100.
            / (2. * self.bin_radius as f32 + 1.);

        // calculate an exponential moving average to prevent aliasing
        let mut alpha = 1.0 - (-self.delta_time / TAU).exp();  // Sample-rate-aware alpha
        self.slow_ema = alpha * amplitude + (1.0 - alpha) * self.slow_ema;
        alpha *= self.trigger_happiness;
        self.fast_ema = alpha * amplitude + (1.0 - alpha) * self.fast_ema;
        self.fast_ema - self.slow_ema
    }
}