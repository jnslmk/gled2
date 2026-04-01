use crate::audio::audio_device_label;
use cpal::{
    DeviceId, SampleFormat, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use kanal::Sender;
use log::debug;
use rtrb::{Consumer, Producer, RingBuffer};
use rustfft::{FftPlanner, num_complex::Complex};
use std::{
    clone::Clone,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};

pub const SAMPLE_RATE: f32 = 48_000.0;
pub const MAX_FREQ: f32 = 24_000.0;

// FFT size - power of 2 for efficient FFT
const WINDOW_SIZE: usize = 2048;
pub const FREQ_BINS: usize = 256;
const STREAM_RESTART_DELAY_MS: u64 = 500;

static DROPPED_SAMPLES: AtomicU64 = AtomicU64::new(0);

pub fn get_fft_bin_index_by_frequency(frequency: f32) -> usize {
    let k = WINDOW_SIZE as f32 * frequency / SAMPLE_RATE;
    k.floor() as usize
}

pub fn max_frequency() -> f32 {
    SAMPLE_RATE / FREQ_BINS as f32
}

pub fn fft_data_u8(fft_data: Vec<f32>) -> [u8; FREQ_BINS * 4] {
    let mut fft_data_u8 = [0u8; FREQ_BINS * 4];
    for (i, &value) in fft_data.iter().enumerate() {
        let bytes = value.to_le_bytes();
        fft_data_u8[i * 4..(i + 1) * 4].copy_from_slice(&bytes);
    }
    fft_data_u8
}

#[derive(Debug)]
pub struct FFTAudioSource {
    running: Arc<AtomicBool>,
}

impl Drop for FFTAudioSource {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

impl FFTAudioSource {
    pub fn start(device_id: DeviceId, fft_tx: Sender<[f32; FREQ_BINS]>) -> Self {
        let running = Arc::new(AtomicBool::new(true));

        std::thread::Builder::new()
            .name("gled:audio:capture".to_string())
            .spawn({
                let running = running.clone();
                move || {
                    #[cfg(feature = "profiling")]
                    profiling::register_thread!("audio:capture");

                    let host = cpal::default_host();

                    while running.load(Ordering::Relaxed) {
                        let device = match host.device_by_id(&device_id) {
                            Some(device) => {
                                log::info!(
                                    "Using audio input device: {}",
                                    device
                                        .description()
                                        .map(|description| audio_device_label(
                                            &device_id,
                                            &description
                                        ))
                                        .unwrap_or_else(|_| "Unknown".to_string())
                                );
                                device
                            }
                            None => {
                                log::warn!(
                                    "No audio input device found, retrying in {} ms",
                                    STREAM_RESTART_DELAY_MS
                                );
                                std::thread::sleep(Duration::from_millis(STREAM_RESTART_DELAY_MS));
                                continue;
                            }
                        };

                        let config = match device.default_input_config() {
                            Ok(config) => config,
                            Err(err) => {
                                log::error!("Failed to get default input config: {}", err);
                                std::thread::sleep(Duration::from_millis(STREAM_RESTART_DELAY_MS));
                                continue;
                            }
                        };

                        log::info!("Audio input config: {:?}", config);
                        let channels = config.channels() as usize;
                        let stream_config = StreamConfig::from(config.clone());
                        let stream_failed = Arc::new(AtomicBool::new(false));

                        let (mut producer, consumer) = RingBuffer::<f32>::new(WINDOW_SIZE * 2);
                        let mut processor = FFTProcessor::new(consumer, fft_tx.clone());

                        let stream = match config.sample_format() {
                            SampleFormat::F32 => {
                                let error_flag = stream_failed.clone();
                                device.build_input_stream(
                                    &stream_config,
                                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                                        push_sample(Vec::from(data), channels, &mut producer);
                                    },
                                    move |err| {
                                        log::error!("Audio stream error: {}", err);
                                        error_flag.store(true, Ordering::Relaxed);
                                    },
                                    None,
                                )
                            }
                            SampleFormat::I16 => {
                                let error_flag = stream_failed.clone();
                                device.build_input_stream(
                                    &stream_config,
                                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                                        let f32_data: Vec<f32> = data
                                            .iter()
                                            .map(|&sample| normalize_i16_sample(sample))
                                            .collect();
                                        push_sample(f32_data, channels, &mut producer);
                                    },
                                    move |err| {
                                        log::error!("Audio stream error: {}", err);
                                        error_flag.store(true, Ordering::Relaxed);
                                    },
                                    None,
                                )
                            }
                            SampleFormat::U16 => {
                                let error_flag = stream_failed.clone();
                                device.build_input_stream(
                                    &stream_config,
                                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                                        let f32_data: Vec<f32> = data
                                            .iter()
                                            .map(|&sample| normalize_u16_sample(sample))
                                            .collect();
                                        push_sample(f32_data, channels, &mut producer);
                                    },
                                    move |err| {
                                        log::error!("Audio stream error: {}", err);
                                        error_flag.store(true, Ordering::Relaxed);
                                    },
                                    None,
                                )
                            }
                            _ => {
                                log::error!(
                                    "Unsupported sample format: {:?}",
                                    config.sample_format()
                                );
                                std::thread::sleep(Duration::from_millis(STREAM_RESTART_DELAY_MS));
                                continue;
                            }
                        };

                        let stream = match stream {
                            Ok(stream) => stream,
                            Err(err) => {
                                log::error!("Failed to build audio stream: {}", err);
                                std::thread::sleep(Duration::from_millis(STREAM_RESTART_DELAY_MS));
                                continue;
                            }
                        };
                        if let Err(err) = stream.play() {
                            log::error!("Failed to start audio stream: {}", err);
                            std::thread::sleep(Duration::from_millis(STREAM_RESTART_DELAY_MS));
                            continue;
                        }
                        log::info!("Audio stream started successfully");

                        while running.load(Ordering::Relaxed)
                            && !stream_failed.load(Ordering::Relaxed)
                        {
                            processor.process_audio_samples();
                        }

                        if stream_failed.load(Ordering::Relaxed) && running.load(Ordering::Relaxed)
                        {
                            log::warn!(
                                "Audio stream stopped unexpectedly, retrying in {} ms",
                                STREAM_RESTART_DELAY_MS
                            );
                            std::thread::sleep(Duration::from_millis(STREAM_RESTART_DELAY_MS));
                        }
                    }

                    log::info!("Audio capture stopping...");
                }
            })
            .expect("Could not spawn audio capture thread");

        Self { running }
    }
}

fn normalize_i16_sample(sample: i16) -> f32 {
    sample as f32 / i16::MAX as f32
}

fn normalize_u16_sample(sample: u16) -> f32 {
    (sample as f32 / u16::MAX as f32) * 2.0 - 1.0
}

pub fn push_sample(data: Vec<f32>, channels: usize, producer: &mut Producer<f32>) {
    // Convert interleaved samples to mono by averaging channels
    #[cfg(feature = "profiling")]
    puffin::profile_function!("audio:push_sample");
    for chunk in data.chunks(channels) {
        let mono_sample = if channels > 1 {
            chunk.iter().sum::<f32>() / channels as f32
        } else {
            chunk[0]
        };
        producer.push(mono_sample).unwrap_or_else(|_| {
            DROPPED_SAMPLES.fetch_add(1, Ordering::Relaxed);
        })
    }
}

struct FFTProcessor {
    consumer: Consumer<f32>,
    fft: Arc<dyn rustfft::Fft<f32>>,
    fft_tx: Sender<[f32; FREQ_BINS]>,
}

impl FFTProcessor {
    pub fn new(consumer: Consumer<f32>, fft_tx: Sender<[f32; FREQ_BINS]>) -> Self {
        // Create FFT planner
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(WINDOW_SIZE);
        Self {
            consumer,
            fft,
            fft_tx,
        }
    }

    pub fn process_audio_samples(&mut self) {
        // When we have enough samples, perform FFT
        let Ok(chunk) = self.consumer.read_chunk(WINDOW_SIZE) else {
            std::thread::sleep(std::time::Duration::from_millis(1));
            return;
        };

        #[cfg(feature = "profiling")]
        puffin::profile_scope!("audio:process_audio_samples");
        // move out of buffer
        let mut samples = [0f32; WINDOW_SIZE];
        let (h, t) = chunk.as_slices();
        samples[..h.len()].copy_from_slice(h);
        samples[h.len()..].copy_from_slice(t);
        // only consume a fraction of the buffer for overlap
        chunk.commit(WINDOW_SIZE / 4);

        // Convert to complex numbers (imaginary part is 0 for real input)
        let mut complex_samples: Vec<Complex<f32>> =
            samples.iter().map(|&s| Complex::new(s, 0.0)).collect();

        // Apply window function (Hanning window) to reduce spectral leakage
        for (i, sample) in complex_samples.iter_mut().enumerate() {
            let window = 0.5
                * (1.0
                    - (2.0 * std::f32::consts::PI * i as f32 / (WINDOW_SIZE as f32 - 1.0)).cos());
            sample.re *= window;
        }

        // Perform FFT
        self.fft.process(&mut complex_samples);

        // Calculate magnitude for each frequency bin
        // Only use first FREQ_BINS (Nyquist frequency limit)
        let mut linear_magnitudes = Vec::with_capacity(FREQ_BINS);

        for sample in complex_samples.iter().take(FREQ_BINS) {
            // Calculate magnitude (norm of complex number)
            let magnitude = sample.norm();
            linear_magnitudes.push(magnitude);
        }

        //// Apply logarithmic frequency scaling
        //// Map linear FFT bins to logarithmic frequency bins
        let mut magnitudes = vec![0.0f32; FREQ_BINS];
        for (output_bin, magnitude_slot) in magnitudes.iter_mut().enumerate() {
            // Map output bin to logarithmic frequency scale
            // Use logarithmic mapping: log(freq) = log(min) + (log(max) - log(min)) * (bin / total_bins)
            let min_freq = 0f32;
            let max_freq: f32 = 400.0;
            let _log_min = min_freq.ln();
            let _log_max = max_freq.ln();
            //let log_freq = log_min + (log_max - log_min) * ;
            let linear_freq = min_freq
                + (max_freq - min_freq) * curved_log(output_bin as f32 / FREQ_BINS as f32, -8.0);

            // Find the corresponding linear bin(s) and interpolate
            let linear_bin = linear_freq - 1.0;
            let lower_bin = linear_bin.floor() as usize;
            let upper_bin = (linear_bin.ceil() as usize).min(FREQ_BINS - 1);
            let fraction = linear_bin - lower_bin as f32;

            if lower_bin < FREQ_BINS {
                let lower_mag = linear_magnitudes[lower_bin];
                let upper_mag = if upper_bin < FREQ_BINS && upper_bin != lower_bin {
                    linear_magnitudes[upper_bin]
                } else {
                    lower_mag
                };
                let scaled_mag = lower_mag * (1.0 - fraction) + upper_mag * fraction;
                *magnitude_slot = scaled_mag;
                // Update max_magnitude with the scaled value
            }
        }

        let sample = magnitudes.try_into().unwrap();
        if let Err(error) = self.fft_tx.try_send(sample) {
            debug!("Error sending fft: {error}")
        };
    }
}

fn curved_log(x: f32, k: f32) -> f32 {
    if k == 0.0 {
        return x;
    }
    f32::ln(1.0 + (f32::exp(k) - 1.0) * x) / k
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_i16_sample_maps_full_scale_range() {
        assert_eq!(normalize_i16_sample(0), 0.0);
        assert!((normalize_i16_sample(i16::MAX) - 1.0).abs() < f32::EPSILON);
        assert!(normalize_i16_sample(i16::MIN) <= -1.0);
    }

    #[test]
    fn normalize_u16_sample_maps_full_scale_range() {
        assert!((normalize_u16_sample(0) + 1.0).abs() < f32::EPSILON);
        assert!(normalize_u16_sample(u16::MAX) >= 1.0);
        assert!(normalize_u16_sample(u16::MAX / 2).abs() < 0.001);
    }
}
