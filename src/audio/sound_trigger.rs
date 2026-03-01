use crate::audio::fft::{FREQ_BINS, MAX_FREQ};
use crate::audio::SOUND_TRIGGER_SAMPLE_INTERVAL_MS;
use ndarray::{s, Array1};
use rustfft::num_traits::Float;
use serde::{Deserialize, Serialize};
use std::ops::Mul;

const MAX_BUFFER_LENGTH: usize = 1001;

#[derive(Debug, Clone, PartialEq, Copy, Serialize, Deserialize)]
pub struct SoundTriggerParams {
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
    pub averaging_samples: usize,
    echo_samples: usize,
}
impl SoundTriggerParams {
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
            averaging_samples: 10,
            echo_samples: 20,
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
        self.gate_deactivation_threshold = self.gate_activation_threshold * 0.9;
        //self.averaging_samples = (100f32.mul(averaging_time) as usize);
    }
}

impl Default for SoundTriggerParams {
    fn default() -> Self {
        SoundTriggerParams::new(
            8520.,
            990.,
            0.1,
            0.2,
            0.5,
            1.,
            1.,
            0.5,
        )
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
pub struct SoundTrigger {
    pub phase: AdsrPhase,
    pub gate_active: bool,
    pub params: SoundTriggerParams,
    prev_params: SoundTriggerParams,
    pub impulse: f32,
    pub delta_time: f32,
    pub spectrum: Array1<f32>,
    pub current_level: f32,
    sample_buffer: PrimitiveRingBuffer<Array1<f32>>,
    running_sum: Array1<f32>,
    prev_averaging_samples: usize,
    max_impulse: f32,
    echo_buffer: PrimitiveRingBuffer<f32>,
    running_impulse_sum: f32,
}

impl Default for SoundTrigger {
    fn default() -> Self {
        SoundTrigger::new(
            SoundTriggerParams::default(),
            SOUND_TRIGGER_SAMPLE_INTERVAL_MS as f32 / 1000.,
        )
    }
}

impl SoundTrigger {
    pub fn new(params: SoundTriggerParams, delta_time: f32) -> Self {
        Self {
            delta_time,
            gate_active: false,
            spectrum: Array1::zeros(FREQ_BINS),
            phase: AdsrPhase::Idle,
            params,
            prev_params: params,
            current_level: 0.0,
            impulse: 0.0,
            sample_buffer: PrimitiveRingBuffer::new(Array1::zeros(FREQ_BINS), MAX_BUFFER_LENGTH),
            running_sum: Array1::zeros(FREQ_BINS),
            prev_averaging_samples: params.averaging_samples,
            max_impulse: 0.00001,
            echo_buffer: PrimitiveRingBuffer::new(0.0, MAX_BUFFER_LENGTH),
            running_impulse_sum: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.phase = AdsrPhase::Idle;
        self.gate_active = false;
        self.current_level = 0.0;
        self.impulse = 0.0;
        self.sample_buffer = PrimitiveRingBuffer::new(Array1::zeros(FREQ_BINS), MAX_BUFFER_LENGTH);
        self.running_sum = Array1::zeros(FREQ_BINS);
        self.spectrum = Array1::zeros(FREQ_BINS);
    }

    pub fn tick(&mut self, root_sample: [f32; FREQ_BINS]) {
        #[cfg(feature = "profiling")]
        puffin::profile_function!("ReactiveSignal::tick");
        self.spectrum = self.compute_running_average(root_sample);
        //self.spectrum = self.spectrum.map(|x|{ 1.0 + (10.0 * self.params.sensitivity + 1.0) * x.mul(5.0).log10() });
        self.spectrum.map_inplace(|x|{ *x = x.mul(self.delta_time * 3.0).clamp(0.0, f32::infinity());});

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

        self.impulse = self.highpass(self.impulse);
        self.impulse =self.normalize(self.impulse);
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

    pub fn compute_running_average(&mut self, current_sample: [f32; FREQ_BINS]) -> Array1<f32> {
        // idea of the running sum:
        // the following condition shall always hold:
        // running_sum == (current_buffer_index-averaging_samples..=current_buffer_index).folding_sum(|i| buffer[i])
        if self.prev_averaging_samples != self.params.averaging_samples {
            // dynamically adapt the running sample to the new length
            // let delta = self.params.averaging_samples as i32 - self.prev_averaging_samples as i32;
            // if delta > 0 {
            //     // the new buffer is longer:
            //     // add the samples from the *new* last index until the *old* last index to the running sum
            //     // including the new last index, excluding the old last index
            //     (self.prev_averaging_samples+1..self.params.averaging_samples+1).for_each(|i|
            //         self.running_sum += &self.sample_buffer.get_from_offset(-(i as i32))
            //     );
            // } else {
            //     // the new buffer is shorter:
            //     // remove the samples from the *old* last index until the *new* last index from the running sum
            //     // including the old last index, excluding the new last index
            //     (self.params.averaging_samples..self.prev_averaging_samples).for_each(|i|
            //         self.running_sum -= &self.sample_buffer.get_from_offset(-(i as i32))
            //     );
            // }

            // the non-dynamic variant of this:
             self.running_sum = (0..=self.params.averaging_samples).map(|i|{
                 self.sample_buffer.get_from_offset(i as i32)
             }).fold(Array1::zeros(FREQ_BINS), |acc, x| acc + x);

            self.prev_averaging_samples = self.params.averaging_samples;
        }

        let current_rms_sample = Array1::from_vec(current_sample.to_vec());

        // remove the oldest sample from the buffer
        let remove = self.sample_buffer.get_from_offset(-(self.params.averaging_samples as i32));
        self.running_sum = &self.running_sum - &remove;
        // advance the buffer
        self.sample_buffer.progress();
        // add the new sample to the buffer
        self.sample_buffer.insert(current_rms_sample.clone());
        self.running_sum = &self.running_sum + &current_rms_sample;

        // use a small decay here to counter the accumulation of errors
        //self.running_sum = &self.running_sum * 0.98;
        &self.running_sum / self.params.averaging_samples as f32
    }

    fn normalize(&mut self, sample: f32) -> f32{
        // reset on change
        if self.params.bin_radius != self.prev_params.bin_radius
            || self.params.center_bin != self.prev_params.center_bin
            || self.params.averaging_samples != self.prev_params.averaging_samples
            || self.prev_params.sensitivity != self.params.sensitivity
        {
            self.max_impulse = 0.00001;
            self.prev_params = self.params;
        }
        self.max_impulse = self.max_impulse.max(sample);
        self.impulse / self.max_impulse
    }


    fn highpass(&mut self, x: f32) -> f32 {
        let remove = self.echo_buffer.get_from_offset(-(self.params.echo_samples as i32));
        self.running_impulse_sum = &self.running_impulse_sum - &remove;
        // advance the buffer
        self.echo_buffer.progress();
        // add the new sample to the buffer
        self.echo_buffer.insert(x);
        self.running_impulse_sum = &self.running_impulse_sum + &x;
        let y = &self.running_impulse_sum / (2.0*self.params.echo_samples as f32);
        x - y
    }
}

#[derive(Clone, Debug, PartialEq)]
struct PrimitiveRingBuffer<T> where T: Clone {
    inner: Vec<T>,
    capacity: usize,
    front: usize,
}

impl<T> PrimitiveRingBuffer<T> where T: Clone {
    pub fn new(default_elem: T, capacity: usize) -> Self{
        Self{
            inner: Vec::from_iter(std::iter::repeat(default_elem).take(capacity)),
            capacity,
            front: capacity.saturating_sub(1),
        }
    }
    pub fn get_from_offset(&self, offset: i32) -> T {
        self.inner[
            (self.front as i32 + offset)
                .rem_euclid(self.capacity as i32) as usize
            ].clone()
    }
    pub fn insert(&mut self, elem: T) {
        self.inner[self.front] = elem;
    }

    pub fn progress(&mut self) {
        self.front = (self.front + 1) % self.capacity;
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn run_for_with_impulse_at(run_for: usize, at: usize, impulse: f32) -> SoundTrigger {
        let mut signal = SoundTrigger::new(SoundTriggerParams::default(), 0.01);
        signal.params.averaging_samples = 1;
        for i in 0..run_for {
            let data = if i == at {[impulse; FREQ_BINS]} else {[0f32; FREQ_BINS]};
            signal.tick(data);
            eprintln!("{:?}", signal.running_sum);
            eprintln!("--------------")
        }
        signal
    }

    #[test]
    fn test_running_sum_removal(){
        let signal = run_for_with_impulse_at(4, 1, 1.0);
        assert_eq!(signal.running_sum, Array1::from_vec(vec![0f32; FREQ_BINS]));
    }

    #[test]
    fn test_wraparound(){
            let mut signal = SoundTrigger::new(SoundTriggerParams::default(), 0.01);
            signal.params.averaging_samples = 1;
            for i in 0..(MAX_BUFFER_LENGTH +10) {
                signal.tick([1f32; FREQ_BINS]);
                eprintln!("running_rms_sum = {:?}", signal.running_sum);
                eprintln!("--------------");
                if i >= 2 {
                    assert_eq!(signal.running_sum, Array1::from_vec(vec![2f32; FREQ_BINS]))
                }
            }
        }
}