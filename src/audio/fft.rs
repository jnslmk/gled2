use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{DeviceId, SampleFormat, StreamConfig, SupportedStreamConfig};
use rustfft::num_traits::Pow;
use rustfft::{num_complex::Complex, FftPlanner};
use std::clone::Clone;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;
use crossbeam_channel::Sender;
use rtrb::{Consumer, Producer, RingBuffer};
use tokio_util::sync::CancellationToken;

pub const SAMPLE_RATE: f32 = 48_000.0;
pub const MAX_FREQ: f32 = 24_000.0;

// FFT size - power of 2 for efficient FFT
const WINDOW_SIZE: usize = 256;
pub const FREQ_BINS: usize = 256;

pub const RMS_BUFFER_SIZE: usize = 100;
static RMS_INDEX: AtomicUsize = AtomicUsize::new(0);

static DROPPED_SAMPLES: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct RootSample {
    pub data: [f32; FREQ_BINS],
    pub index: usize,
}

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

pub async fn start(device_id: DeviceId, fft_tx: Sender<RootSample>, cancel_token: CancellationToken) {
    log::info!("Starting audio capture thread");
    #[cfg(feature = "profiling")]
    profiling::register_thread!("audio:capture");
    RMS_INDEX.store(0, Relaxed);
    log::info!("Starting FFT thread");

    let host = cpal::default_host();
    let device = host.device_by_id(&device_id);

    let device = match device {
        Some(device) => {
            log::info!(
                "Using audio input device: {}",
                device
                    .description()
                    .map(|desc| desc.name().to_string())
                    .unwrap_or_else(|_| "Unknown".to_string())
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
    let channels = config.channels() as usize;


    let (mut producer, mut consumer) = RingBuffer::<f32>::new(48_0000);
    let mut processor = FFTProcessor::new(consumer, fft_tx, );

    let stream = match config.sample_format() {
        SampleFormat::F32 => {
            device.build_input_stream(
                &StreamConfig::from(config),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    push_sample(Vec::from(data), channels, &mut producer);
                },
                |err| {
                    log::error!("Audio stream error: {}", err);
                },
                None,
            )
        }
        SampleFormat::I16 => {
            device.build_input_stream(
                &StreamConfig::from(config),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    let f32_data: Vec<f32> = data.iter().map(|&s| s as f32 / MAX_FREQ).collect();
                    push_sample(f32_data, channels, &mut producer);
                },
                |err| {
                    log::error!("Audio stream error: {}", err);
                },
                None,
            )
        }
        SampleFormat::U16 => {
            device.build_input_stream(
                &StreamConfig::from(config),
                move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    let f32_data: Vec<f32> =
                        data.iter().map(|&s| (s as f32 / 32768.0) - 1.0).collect();
                    push_sample(f32_data, channels, &mut producer);
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
            let cancel_processing_token = cancel_token.clone();
            tokio::spawn(async move {
                loop {
                    if cancel_processing_token.is_cancelled() {
                        log::info!("FFT thread cancelled");
                        break;
                    }
                    processor.process_audio_samples();
                    tokio::task::yield_now().await;
                }
            });
            loop {
                if cancel_token.is_cancelled() {
                    log::info!("Audio capture cancelled");
                    break;
                }
                tokio::task::yield_now().await;
            }
        }
        Err(err) => {
            log::error!("Failed to build audio stream: {}", err);
        }
    }
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
        producer.push(mono_sample).unwrap_or_else(|_| {DROPPED_SAMPLES.fetch_add(1, Ordering::Relaxed);})
    }
}

struct FFTProcessor{
    consumer: Consumer<f32>,
    fft: Arc<dyn rustfft::Fft<f32>>,
    fft_tx: Sender<RootSample>,
}

impl FFTProcessor {

    pub fn new(
        consumer: Consumer<f32>,
        fft_tx: Sender<RootSample>,
    ) -> Self {
        // Create FFT planner
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(WINDOW_SIZE);
        Self{
            consumer,
            fft,
            fft_tx,
        }
    }

    pub fn process_audio_samples(
        &mut self,
    ) {
        // When we have enough samples, perform FFT
        if let Ok(chunk) = self.consumer.read_chunk(WINDOW_SIZE) {
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
                    * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (WINDOW_SIZE as f32 - 1.0)).cos());
                sample.re *= window;
            }

            // Perform FFT
            self.fft.process(&mut complex_samples);

            // Calculate magnitude for each frequency bin
            // Only use first FREQ_BINS (Nyquist frequency limit)
            let mut linear_magnitudes = Vec::with_capacity(FREQ_BINS);

            for sample in complex_samples.iter().take(FREQ_BINS) {
                // Calculate magnitude (norm of complex number)
                let magnitude = sample.norm().pow(2);
                linear_magnitudes.push(magnitude);
            }

            //// Apply logarithmic frequency scaling
            //// Map linear FFT bins to logarithmic frequency bins
            //let mut magnitudes = vec![0.0f32; FREQ_BINS];
            //for (output_bin, magnitude_slot) in magnitudes.iter_mut().enumerate() {
            //    // Map output bin to logarithmic frequency scale
            //    // Use logarithmic mapping: log(freq) = log(min) + (log(max) - log(min)) * (bin / total_bins)
            //    let min_freq = 1.0f32;
            //    let max_freq = FREQ_BINS as f32;
            //    let log_min = min_freq.ln();
            //    let log_max = max_freq.ln();
            //    let log_freq = log_min + (log_max - log_min) * (output_bin as f32 / FREQ_BINS as f32);
            //    let linear_freq = log_freq.exp();
            //
            //    // Find the corresponding linear bin(s) and interpolate
            //    let linear_bin = linear_freq - 1.0;
            //    let lower_bin = linear_bin.floor() as usize;
            //    let upper_bin = (linear_bin.ceil() as usize).min(FREQ_BINS - 1);
            //    let fraction = linear_bin - lower_bin as f32;
            //
            //    if lower_bin < FREQ_BINS {
            //        let lower_mag = linear_magnitudes[lower_bin];
            //        let upper_mag = if upper_bin < FREQ_BINS && upper_bin != lower_bin {
            //            linear_magnitudes[upper_bin]
            //        } else {
            //            lower_mag
            //        };
            //        let scaled_mag = lower_mag * (1.0 - fraction) + upper_mag * fraction;
            //        *magnitude_slot = scaled_mag;
            //        // Update max_magnitude with the scaled value
            //    }
            //}

            let mut current_index = RMS_INDEX.load(Relaxed);
            current_index = (current_index + 1) % RMS_BUFFER_SIZE;
            RMS_INDEX.store(current_index,Relaxed);
            let sample = RootSample {
                data: linear_magnitudes.try_into().unwrap(),
                index: current_index,
            };
            let _ = self.fft_tx.send(sample);
        }
    }
}
