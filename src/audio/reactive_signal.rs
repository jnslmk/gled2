use crate::audio::fft::{RootSample, FREQ_BINS, MAX_FREQ, RMS_BUFFER_SIZE};
use crate::audio::ADSR_SAMPLE_INTERVAL_MS;
use ndarray::{concatenate, s, Array1, Array2, Axis};
use rustfft::num_traits::Float;
use serde::{Deserialize, Serialize};
use std::ops::Mul;

#[derive(Debug, Clone, PartialEq, Copy, Serialize, Deserialize)]
pub struct AdsrParams {
    center_bin: usize,
    bin_radius: usize,
    pub attack_duration: f32,  // in seconds
    pub decay_duration: f32,   // in seconds
    pub sustain_level: f32,    // 0.0 to 1.0
    pub release_duration: f32, // in seconds
    /// Threshold to consider the continuous input "ON"
    pub sensitivity: f32,
    pub gate_activation_threshold: f32,
    pub gate_deactivation_threshold: f32,
    pub rms_length: usize,
}
impl AdsrParams {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        f_center: f32,
        f_radius: f32,
        attack_duration: f32,
        decay_duration: f32,
        sustain_level: f32,
        release_duration: f32,
        sensitivity: f32,
        gate_threshold: f32,
    ) -> Self {
        let mut ret = Self {
            center_bin: 0,
            bin_radius: 0,
            attack_duration,
            decay_duration,
            sustain_level,
            release_duration,
            sensitivity,
            gate_activation_threshold: gate_threshold,
            gate_deactivation_threshold: gate_threshold * 0.8,
            rms_length: 10,
        };
        ret.set_filter_tune(f_center, f_radius);
        ret
    }
    pub fn set_filter_tune(&mut self, f_center: f32, f_radius: f32) {
        let f_per_bin = MAX_FREQ / FREQ_BINS as f32;
        let center_bin = ((f_center / f_per_bin).round() as usize).min(FREQ_BINS - 1);
        let bin_radius = (f_radius / f_per_bin).round() as usize;
        self.center_bin = center_bin;
        self.bin_radius = bin_radius;
        self.gate_deactivation_threshold = self.gate_activation_threshold * 0.8;
    }
}

impl Default for AdsrParams {
    fn default() -> Self {
        AdsrParams::new(8520., 990., 0.1, 0.2, 0.5, 1., 1., 0.5)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum AdsrPhase {
    #[default]
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ReactiveSignal {
    pub phase: AdsrPhase,
    pub gate_active: bool,
    pub params: AdsrParams,
    pub impulse: f32,
    pub delta_time: f32,
    pub spectrum: Array1<f32>,
    pub current_level: f32,
    prev_gamma: Array1<f32>,
    rms_buffer: Array2<f32>,
    running_rms_sum: Array1<f32>,
}

impl Default for ReactiveSignal {
    fn default() -> Self {
        ReactiveSignal::new(
            AdsrParams::default(),
            ADSR_SAMPLE_INTERVAL_MS as f32 / 1000.,
        )
    }
}

// TODO cleanup
impl ReactiveSignal {
    pub fn new(params: AdsrParams, delta_time: f32) -> Self {
        Self {
            delta_time,
            gate_active: false,
            spectrum: Array1::zeros(FREQ_BINS),
            phase: AdsrPhase::Idle,
            params,
            current_level: 0.0,
            impulse: 0.0,
            prev_gamma: Array1::zeros(FREQ_BINS),
            rms_buffer: Array2::zeros((RMS_BUFFER_SIZE, FREQ_BINS)),
            running_rms_sum: Array1::ones(FREQ_BINS),
        }
    }

    pub fn reset(&mut self) {
        self.phase = AdsrPhase::Idle;
        self.gate_active = false;
        self.current_level = 0.0;
        self.impulse = 0.0;
        self.prev_gamma = Array1::zeros(FREQ_BINS);
        self.rms_buffer = Array2::zeros((RMS_BUFFER_SIZE, FREQ_BINS));
        self.running_rms_sum = Array1::ones(FREQ_BINS);
        self.spectrum = Array1::zeros(FREQ_BINS);
    }

    pub fn tick(&mut self, root_sample: RootSample) {
        #[cfg(feature = "profiling")]
        puffin::profile_function!("ReactiveSignal::tick");
        self.rms(root_sample.data, root_sample.index);
        self.spectrum = self.running_rms_sum.map(|x|{ 1.0 + (10.0 * self.params.sensitivity + 1.0) * x.log10() });
        self.spectrum.map_inplace(|x|{ *x = x.mul(self.delta_time * 10.0).clamp(0.0, f32::infinity());});

        let max_f = (self.params.center_bin + self.params.bin_radius).clamp(0, FREQ_BINS - 1);
        self.impulse = (self
            .spectrum
            .slice(s![self
                .params
                .center_bin
                .saturating_sub(self.params.bin_radius)
                .clamp(1, FREQ_BINS - 1)..max_f])
            .iter()
            .map(|x| x.powf(2.))
            .sum::<f32>()
            / (2. * self.params.bin_radius as f32))
            .sqrt()
            .mul(4.0);
        self.tick_adsr(self.impulse);
    }

    /// Calculate the envelope amplitude at the current timepoint.
    ///
    /// # Arguments
    /// * `input` - A continuous float control signal (Gate).
    pub fn tick_adsr(&mut self, input: f32) -> f32 {
        self.gate_active = if self.gate_active {
            input > self.params.gate_deactivation_threshold
        } else {
            input > self.params.gate_activation_threshold
        };

        // 1. Handle State Transitions
        match self.phase {
            AdsrPhase::Idle => {
                if self.gate_active {
                    self.phase = AdsrPhase::Attack;
                }
            }
            AdsrPhase::Attack => {
                if !self.gate_active {
                    self.phase = AdsrPhase::Release;
                } else if self.current_level >= 1.0 {
                    self.phase = AdsrPhase::Decay;
                }
            }
            AdsrPhase::Decay => {
                if !self.gate_active {
                    self.phase = AdsrPhase::Release;
                } else if self.current_level <= self.params.sustain_level {
                    self.phase = AdsrPhase::Sustain;
                }
            }
            AdsrPhase::Sustain => {
                if !self.gate_active {
                    self.phase = AdsrPhase::Release;
                }
                // In Sustain, we just hold the level (handled in calculation step)
            }
            AdsrPhase::Release => {
                if self.gate_active {
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
    pub fn rms(&mut self, current_rms_sample: [f32; FREQ_BINS], current_rms_index: usize) {
        let current_rms_sample = Array1::from_vec(current_rms_sample.to_vec());
        self.rms_buffer.row_mut(current_rms_index).assign(&Array1::from_vec(current_rms_sample.to_vec()));

        let drop_index = (current_rms_index + RMS_BUFFER_SIZE - self.params.rms_length) % RMS_BUFFER_SIZE;
        // add RMS_BUFFER_SIZE to the index before substraction to prevent overflows
        // the following is for implementing wraparound indices
        let divisor =  self.region(0, RMS_BUFFER_SIZE - 1)
            .fold(0f32,|x, y| x.max(*y).max(0.01));
        self.running_rms_sum =self.region(current_rms_index, drop_index).sum_axis(Axis(0)) / divisor;

        self.running_rms_sum.mapv(|x|{x.mul(1.0/(self.params.rms_length as f32)).max(0.000001)}.sqrt());
    }

    fn region(&mut self, current_rms_index: usize, drop_index: usize) -> Array2<f32> {
        if drop_index > current_rms_index {
            // right subinterval
            concatenate![Axis(0), self.rms_buffer.slice(s![drop_index+1..RMS_BUFFER_SIZE, ..]),
            self.rms_buffer.slice(s![..=current_rms_index, ..])]

        } else {
            self.rms_buffer.slice(s![drop_index+1..=current_rms_index, ..]).to_owned()
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn run_for_with_impulse_at(run_for: usize, at: usize, impulse: f32) -> ReactiveSignal {
        let mut signal = ReactiveSignal::new(AdsrParams::default(), 0.01);
        signal.params.rms_length = 2;
        for i in 0..run_for {
            let index = i%RMS_BUFFER_SIZE;
            let data = if i == at {[impulse; FREQ_BINS]} else {[0f32; FREQ_BINS]};
            signal.tick(RootSample{data, index });
            eprintln!("{:?}", signal.spectrum);
            eprintln!("--------------")
        }
        signal
    }

    #[test]
    fn test_running_sum_removal(){
        let signal = run_for_with_impulse_at(4, 1, 1.0);
        assert_eq!(signal.running_rms_sum, Array1::from_vec(vec![0f32; FREQ_BINS]));
    }

    #[test]
    fn test_wraparound(){
            let mut signal = ReactiveSignal::new(AdsrParams::default(), 0.01);
            signal.params.rms_length = 2;
            for i in 0..110 {
                let index = i%RMS_BUFFER_SIZE;
                signal.tick(RootSample{data: [1f32; FREQ_BINS], index});
                eprintln!("index = {}", index);
                eprintln!("running_rms_sum = {:?}", signal.running_rms_sum);
                eprintln!("--------------");
                if i >= 2 {
                    assert_eq!(signal.running_rms_sum, Array1::from_vec(vec![2f32; FREQ_BINS]))
                }
            }
        }
}