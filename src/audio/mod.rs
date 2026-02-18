pub mod adsr_editor;
pub mod reactive_signal;
pub mod fft;
pub mod device_id_serde;

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
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use futures::future::{join_all, JoinAll};
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use tokio::time::interval;
use tokio::{runtime};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub static REACTIVE_SIGNAL_THREAD: Lazy<RwLock<ReactiveSignalThread>> =
    Lazy::new(|| RwLock::new(ReactiveSignalThread::new()));
static ADSR_SAMPLE_INTERVAL_MS: u64 = 10;
pub static AUDIO_DEVICES: Lazy<Mutex<Vec<(DeviceId, DeviceDescription)>>> = Lazy::new(|| Mutex::new(Vec::new()));

#[derive(Debug)]
pub struct AudioPool {
    runtime: Runtime,
    fft_tx: Sender<RootSample>,
    fft_cancel: Option<CancellationToken>,
    pub selected_device: Option<DeviceId>,
}

impl AudioPool {
    pub fn init() -> Self {
        #[cfg(feature = "profiling")]
        puffin::profile_function!("AudioPool::init");
        let runtime = runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .thread_name("gled_audio_pool")
            .enable_time()
            .build().expect("Failed to create audio thread pool");

        let selected_device = None;
        let (fft_tx, receiver) = crossbeam_channel::bounded(3);

        runtime.spawn(start_reactive_sound_thread(receiver));
        runtime.spawn(audio_device_info_loop());
        let mut ret = Self{runtime, fft_cancel: None, selected_device, fft_tx};
        ret.restart_fft();
        ret
    }

    pub fn restart_fft(&mut self){
        if let Some(fft_cancel) = self.fft_cancel.take() {
            // wait for the thread to stop
            fft_cancel.cancel();
        }

        if let Some(device_id) = self.selected_device.clone() {
            let cancel_token = CancellationToken::new();
            self.runtime.spawn(fft::start(device_id, self.fft_tx.clone(), cancel_token.clone()));
            self.fft_cancel = Some(cancel_token);
        }
        else {
            log::info!("No audio input device selected, audio analysis disabled");
            self.fft_cancel = None;
            REACTIVE_SIGNAL_THREAD.write().unwrap().reset();
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
                let mut signal = signal.lock().unwrap();
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
            .map(|signal| signal.lock().unwrap().current_level)
            .unwrap_or(0.0)
    }
}

impl Hash for ReactiveSignalHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.uuid.hash(state);
    }
}

pub struct ReactiveSignalThread {
    signals: HashMap<Uuid,Arc<Mutex<ReactiveSignal>>>,
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
        self.signals.insert(uuid, Arc::new(Mutex::new(signal)));
        ReactiveSignalHandle {
            uuid,
            params: Arc::new(Mutex::new(adsr_params)),
        }
    }

    fn tick(&mut self, root_sample: RootSample) -> JoinAll<JoinHandle<()>> {
        let handles: Vec<JoinHandle<()>> = self.signals
            .values_mut()
            .map(move |signal| {
                let signal = Arc::clone(signal);
                let sample_copy = root_sample.clone();
                tokio::spawn(async move {
                    let mut signal_guard = signal.lock().unwrap();
                    signal_guard.tick(sample_copy)
                })
            }).collect();
        join_all(handles)
    }

    fn reset(&mut self) {
        self.signals
            .values_mut()
            .for_each(move |signal| {
                let signal = Arc::clone(signal);
                let mut signal_guard = signal.lock().unwrap();
                signal_guard.reset();
            });
    }
}
pub async fn start_reactive_sound_thread(rx: Receiver<RootSample>) {
    loop {
        {
            // give tokio the opportunity to break the reactive sound thread loop
            tokio::task::yield_now().await;
            #[cfg(feature = "profiling")]
            puffin::profile_scope!("ReactiveSignalThread::waitForSignal");
            let root_sample = match rx.recv_timeout(Duration::from_millis(10)) {
                Ok(root_sample) => root_sample,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => {
                    log::error!("Audio receiver disconnected, shutting down reactive sound thread");
                    break
                },
            };
                tokio::spawn(REACTIVE_SIGNAL_THREAD.write().unwrap().tick(root_sample));
        }
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
