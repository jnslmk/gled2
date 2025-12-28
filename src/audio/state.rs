use atomic_float::AtomicF32;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use egui::mutex::Mutex;
use once_cell::sync::Lazy;
use rustfft::{FftPlanner, num_complex::Complex};
use std::sync::Arc;
use std::sync::atomic::Ordering;

// FFT size - power of 2 for efficient FFT
const FFT_SIZE: usize = 512;
// Number of frequency bins to expose to shader (typically half of FFT_SIZE due to Nyquist)
const FREQ_BINS: usize = 256;

static FFT_DATA: Lazy<Arc<Mutex<Vec<f32>>>> =
    Lazy::new(|| Arc::new(Mutex::new(vec![0.0; FREQ_BINS])));
static FFT_MAX_MAGNITUDE: Lazy<AtomicF32> = Lazy::new(|| AtomicF32::new(0.0));

pub fn get_fft_data() -> Vec<f32> {
    FFT_DATA.lock().clone()
}

pub fn start() {
    log::info!("Starting audio capture thread");

    let host = cpal::default_host();

    let device = match host.default_input_device() {
        Some(device) => {
            log::info!(
                "Using audio input device: {}",
                device.name().unwrap_or_default()
            );
            device
        }
        None => {
            log::warn!("No audio input device found, audio analysis disabled");
            return;
        }
    };

    let config = match device.default_input_config() {
        Ok(config) => config,
        Err(err) => {
            log::error!("Failed to get default input config: {}", err);
            return;
        }
    };

    log::info!("Audio input config: {:?}", config);

    let sample_rate = config.sample_rate().0 as usize;
    let channels = config.channels() as usize;

    // Create a buffer to accumulate samples (wrapped in Arc<Mutex> for thread safety)
    let sample_buffer = Arc::new(Mutex::new(Vec::with_capacity(FFT_SIZE * channels)));

    // Create FFT planner
    let mut planner = FftPlanner::new();
    let fft = Arc::new(planner.plan_fft_forward(FFT_SIZE));

    let fft_data = FFT_DATA.clone();

    let stream = match config.sample_format() {
        SampleFormat::F32 => {
            let sample_buffer = sample_buffer.clone();
            let fft = fft.clone();
            device.build_input_stream(
                &StreamConfig::from(config),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    process_audio_samples(
                        data,
                        channels,
                        &sample_buffer,
                        &fft,
                        &fft_data,
                        sample_rate,
                    );
                },
                |err| {
                    log::error!("Audio stream error: {}", err);
                },
                None,
            )
        }
        SampleFormat::I16 => {
            let sample_buffer = sample_buffer.clone();
            let fft = fft.clone();
            device.build_input_stream(
                &StreamConfig::from(config),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    let f32_data: Vec<f32> = data.iter().map(|&s| s as f32 / 32768.0).collect();
                    process_audio_samples(
                        &f32_data,
                        channels,
                        &sample_buffer,
                        &fft,
                        &fft_data,
                        sample_rate,
                    );
                },
                |err| {
                    log::error!("Audio stream error: {}", err);
                },
                None,
            )
        }
        SampleFormat::U16 => {
            let sample_buffer = sample_buffer.clone();
            let fft = fft.clone();
            device.build_input_stream(
                &StreamConfig::from(config),
                move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    let f32_data: Vec<f32> =
                        data.iter().map(|&s| (s as f32 / 32768.0) - 1.0).collect();
                    process_audio_samples(
                        &f32_data,
                        channels,
                        &sample_buffer,
                        &fft,
                        &fft_data,
                        sample_rate,
                    );
                },
                |err| {
                    log::error!("Audio stream error: {}", err);
                },
                None,
            )
        }
        _ => {
            log::error!("Unsupported sample format: {:?}", config.sample_format());
            return;
        }
    };

    match stream {
        Ok(stream) => {
            if let Err(err) = stream.play() {
                log::error!("Failed to start audio stream: {}", err);
                return;
            }
            log::info!("Audio stream started successfully");
            // Keep the thread alive
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
        Err(err) => {
            log::error!("Failed to build audio stream: {}", err);
        }
    }
}

fn process_audio_samples(
    data: &[f32],
    channels: usize,
    sample_buffer: &Arc<Mutex<Vec<f32>>>,
    fft: &Arc<dyn rustfft::Fft<f32>>,
    fft_data: &Arc<Mutex<Vec<f32>>>,
    _sample_rate: usize,
) {
    let mut buffer = sample_buffer.lock();

    // Convert interleaved samples to mono by averaging channels
    for chunk in data.chunks(channels) {
        let mono_sample = if channels > 1 {
            chunk.iter().sum::<f32>() / channels as f32
        } else {
            chunk[0]
        };
        buffer.push(mono_sample);
    }

    // When we have enough samples, perform FFT
    if buffer.len() >= FFT_SIZE {
        // Take the last FFT_SIZE samples
        let samples: Vec<f32> = buffer[buffer.len() - FFT_SIZE..].to_vec();

        // Convert to complex numbers (imaginary part is 0 for real input)
        let mut complex_samples: Vec<Complex<f32>> =
            samples.iter().map(|&s| Complex::new(s, 0.0)).collect();

        // Apply window function (Hanning window) to reduce spectral leakage
        for (i, sample) in complex_samples.iter_mut().enumerate() {
            let window = 0.5
                * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (FFT_SIZE as f32 - 1.0)).cos());
            sample.re *= window;
        }

        // Perform FFT
        fft.process(&mut complex_samples);

        // Calculate magnitude for each frequency bin
        // Only use first FREQ_BINS (Nyquist frequency limit)
        let mut linear_magnitudes = Vec::with_capacity(FREQ_BINS);
        let mut max_magnitude = 0.0f32;

        for sample in complex_samples.iter().take(FREQ_BINS) {
            // Calculate magnitude (norm of complex number)
            let magnitude = sample.norm();
            max_magnitude = max_magnitude.max(magnitude);
            linear_magnitudes.push(magnitude);
        }

        // Apply logarithmic frequency scaling
        // Map linear FFT bins to logarithmic frequency bins
        let mut magnitudes = vec![0.0f32; FREQ_BINS];
        for output_bin in 0..FREQ_BINS {
            // Map output bin to logarithmic frequency scale
            // Use logarithmic mapping: log(freq) = log(min) + (log(max) - log(min)) * (bin / total_bins)
            let min_freq = 1.0f32;
            let max_freq = FREQ_BINS as f32;
            let log_min = min_freq.ln();
            let log_max = max_freq.ln();
            let log_freq = log_min + (log_max - log_min) * (output_bin as f32 / FREQ_BINS as f32);
            let linear_freq = log_freq.exp();

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
                magnitudes[output_bin] = scaled_mag;
                // Update max_magnitude with the scaled value
                max_magnitude = max_magnitude.max(scaled_mag);
            }
        }

        // Update the global maximum magnitude seen across all samples
        // Use fetch_max to atomically update the maximum
        let mut current_max = FFT_MAX_MAGNITUDE.load(Ordering::Relaxed);
        loop {
            if max_magnitude <= current_max {
                break;
            }
            match FFT_MAX_MAGNITUDE.compare_exchange_weak(
                current_max,
                max_magnitude,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    current_max = max_magnitude;
                    break;
                }
                Err(x) => current_max = x,
            }
        }
        let normalization_max = current_max.max(1e-6); // Avoid division by zero with small epsilon

        // Normalize all magnitudes using the global maximum
        // This ensures consistent scaling across all time
        let scale = 1.0 / normalization_max;

        for magnitude in magnitudes.iter_mut() {
            // Normalize to [0, 1] range using the global maximum
            *magnitude = (*magnitude * scale).clamp(0.0, 1.0);
        }

        // Update shared FFT data
        *fft_data.lock() = magnitudes;

        // Keep only the last FFT_SIZE samples for overlap
        let buffer_len = buffer.len();
        if buffer_len > FFT_SIZE {
            buffer.drain(0..buffer_len - FFT_SIZE);
        }
    }
}
