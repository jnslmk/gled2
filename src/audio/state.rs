use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use egui::mutex::Mutex;
use once_cell::sync::Lazy;
use rustfft::{FftPlanner, num_complex::Complex};
use std::sync::Arc;

// FFT size - power of 2 for efficient FFT
const FFT_SIZE: usize = 512;
// Number of frequency bins to expose to shader (typically half of FFT_SIZE due to Nyquist)
const FREQ_BINS: usize = 256;

static FFT_DATA: Lazy<Arc<Mutex<Vec<f32>>>> =
    Lazy::new(|| Arc::new(Mutex::new(vec![0.0; FREQ_BINS])));
static FFT_PEAK: Lazy<Arc<Mutex<f32>>> = Lazy::new(|| Arc::new(Mutex::new(0.0)));

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
        let mut magnitudes = Vec::with_capacity(FREQ_BINS);
        let mut max_magnitude = 0.0f32;

        for sample in complex_samples.iter().take(FREQ_BINS) {
            // Calculate magnitude (norm of complex number)
            let magnitude = sample.norm();
            max_magnitude = max_magnitude.max(magnitude);
            magnitudes.push(magnitude);
        }

        // Update peak with exponential decay for smooth auto-scaling
        // This creates an adaptive normalization that follows the audio level
        let peak = FFT_PEAK.clone();
        let mut current_peak = peak.lock();
        // Decay the peak slowly, but allow it to rise quickly
        *current_peak = (*current_peak * 0.98 + max_magnitude * 0.02).max(max_magnitude * 0.5);
        let peak_value = *current_peak;
        drop(current_peak);

        // Normalize magnitudes using the peak value
        // Apply square root compression for better dynamic range visualization
        let scale = if peak_value > 0.0 {
            1.0 / peak_value
        } else {
            0.0
        };

        for magnitude in magnitudes.iter_mut() {
            // Apply square root compression (better for audio visualization)
            // then normalize to [0, 1] range
            let compressed = magnitude.sqrt();
            *magnitude = (compressed * scale).clamp(0.0, 1.0);
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
