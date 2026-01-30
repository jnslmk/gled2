pub mod adsr_editor;
pub mod reactive_signal;
pub mod fft;

use crate::audio::fft::RootSample;
use crate::audio::reactive_signal::{AdsrParams, ReactiveSignal};
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{DeviceDescription, DeviceId};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::broadcast;
use tokio::sync::broadcast::Receiver;
use tokio::task::JoinHandle;
use tokio::time::interval;
use tokio::{runtime, time};
use uuid::Uuid;

pub static REACTIVE_SIGNAL_THREAD: Lazy<RwLock<ReactiveSignalThread>> =
    Lazy::new(|| RwLock::new(ReactiveSignalThread::new()));
static ADSR_SAMPLE_INTERVAL_MS: u64 = 10;
pub static FFT_THREAD: Lazy<Mutex<AudioPool>> = Lazy::new(|| Mutex::new(AudioPool::init()));
pub static AUDIO_DEVICES: Lazy<Mutex<Vec<(DeviceId, DeviceDescription)>>> = Lazy::new(|| Mutex::new(Vec::new()));


pub fn start_fft_thread() {
    FFT_THREAD.lock().unwrap().restart_fft();
}

#[derive(Debug)]
pub struct AudioPool {
    runtime: Runtime,
    fft_tx: broadcast::Sender<RootSample>,
    fft_abort_sender: Option<JoinHandle<()>>,
    pub selected_device: Option<DeviceId>,
}

impl AudioPool {
    pub fn init() -> Self {
        let runtime = runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("gled_audio_pool")
            .enable_time()
            .build().expect("Failed to create audio thread pool");

        let host = cpal::default_host();
        let default_device = host.default_input_device();
        let mut selected_device = None;
        if let Some(default_device) = default_device {
            {
                log::info!("Using audio input device: {:?}", default_device.description());
                selected_device = default_device.id().ok();
            }
        }
        else {
            log::warn!("No audio input device found, audio analysis disabled");
        }
        let (fft_tx, receiver) = broadcast::channel(4);

        runtime.spawn(start_reactive_sound_thread(receiver));
        runtime.spawn(audio_device_info_loop());
        Self{runtime, fft_abort_sender: None, selected_device, fft_tx}
    }

    pub fn restart_fft(&mut self){
        if let Some(fft_abort_sender) = self.fft_abort_sender.take() {
            // wait for the thread to stop
            //fft_abort_sender.abort();
        }

        if let Some(device_id) = self.selected_device.clone() {
            let handle = self.runtime.spawn(fft::start(device_id, self.fft_tx.clone()));
            self.fft_abort_sender = Some(handle);
        }
        else {
            log::warn!("No audio input device selected, audio analysis disabled");
            self.fft_abort_sender = None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactiveSignalHandle {
    uuid: Uuid,
    params: Arc<Mutex<AdsrParams>>,
}

impl Drop for ReactiveSignalHandle {
    fn drop(&mut self) {
        REACTIVE_SIGNAL_THREAD
            .write()
            .unwrap()
            .signals
            .remove(&self.uuid);
    }
}

impl PartialEq for ReactiveSignalHandle {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

impl ReactiveSignalHandle {
    pub fn update_params_and_fetch_signal(&self) -> Option<ReactiveSignal> {
        REACTIVE_SIGNAL_THREAD
            .write()
            .unwrap()
            .signals
            .get_mut(&self.uuid)
            .map(|signal| {
                signal.params = *self.params.lock().unwrap();
                signal.clone()
            })
    }
    pub fn level(&self) -> f32 {
        REACTIVE_SIGNAL_THREAD
            .write()
            .unwrap()
            .signals
            .get_mut(&self.uuid)
            .map(|signal| signal.current_level)
            .unwrap_or(0.0)
    }
}

impl Hash for ReactiveSignalHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.uuid.hash(state);
    }
}

pub struct ReactiveSignalThread {
    signals: HashMap<Uuid, ReactiveSignal>,
}

impl ReactiveSignalThread {
    fn new() -> Self {
        Self {
            signals: HashMap::new(),
        }
    }
    pub fn register_reactive_signal(&mut self, adsr_params: AdsrParams) -> ReactiveSignalHandle {
        let uuid = Uuid::new_v4();
        let signal = ReactiveSignal::new(adsr_params, ADSR_SAMPLE_INTERVAL_MS as f32 / 1000.);
        self.signals.insert(uuid, signal);
        ReactiveSignalHandle {
            uuid,
            params: Arc::new(Mutex::new(adsr_params)),
        }
    }
}
pub async fn start_reactive_sound_thread(mut rx: Receiver<RootSample>) {
    let mut interval = interval(Duration::from_millis(ADSR_SAMPLE_INTERVAL_MS));
    loop {
        {
            let root_sample = rx.recv().await.unwrap();

            #[cfg(feature = "profiling")]
            puffin::profile_scope!("ReactiveSignalThread::tick");
            REACTIVE_SIGNAL_THREAD
                .write()
                .unwrap()
                .signals
                .values_mut()
                .for_each(|signal| {
                    signal.tick(root_sample.clone());
                });
        }
        // this delay needs to be long enough to allow the ui thread to copy data in time
        // this may be suboptimal
        interval.tick().await;
    }
}

// poll for audio device changes every second
pub async fn audio_device_info_loop(){
    #[cfg(feature = "profiling")]
    profiling::register_thread!("audio_device_info_loop");
    let mut interval = interval(Duration::from_secs(1));
    loop {
        {
            #[cfg(feature = "profiling")]
            puffin::profile_scope!("audio_device_info_loop");

            let host = cpal::default_host();
            let devices = host.input_devices().expect("Failed to get audio devices");
            let mut device_map = Vec::from_iter(devices.map(|device| (
                device.id().expect("Failed to get audio device id"),
                device.description().expect("Failed to get audio device description"))));
            let mut hasher = DefaultHasher::new();
            device_map.sort_by(|a, b| a.0.hash(&mut hasher).cmp(&b.0.hash(&mut hasher)));

            *AUDIO_DEVICES.lock().unwrap() = device_map;
        }
        interval.tick().await;
    }
}
