pub mod sound_trigger_editor;
pub mod sound_trigger;
pub mod fft;
pub mod device_id_serde;

use crate::audio::fft::FREQ_BINS;
use crate::audio::sound_trigger::{SoundTriggerParams, SoundTrigger};
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{DeviceDescription, DeviceId};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash};
use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::thread::sleep;
use std::time::Duration;
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use tokio::runtime::Runtime;
use tokio::runtime;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub static SOUND_TRIGGER_THREAD: Lazy<RwLock<SoundTriggeThread>> =
    Lazy::new(|| RwLock::new(SoundTriggeThread::new()));
static SOUND_TRIGGER_SAMPLE_INTERVAL_MS: u64 = 10;
pub static AUDIO_DEVICES: Lazy<Mutex<Vec<(DeviceId, DeviceDescription)>>> = Lazy::new(|| Mutex::new(Vec::new()));

#[derive(Debug)]
pub struct AudioPool {
    runtime: Runtime,
    fft_tx: Sender<[f32; FREQ_BINS]>,
    fft_cancel: Option<CancellationToken>,
    pub selected_device: Option<DeviceId>,
}

impl AudioPool {
    pub fn init() -> Self {
        #[cfg(feature = "profiling")]
        puffin::profile_function!("AudioPool::init");
        let runtime = runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .thread_name("gled_audio_pool")
            .enable_time()
            .build().expect("Failed to create audio thread pool");

        let selected_device = None;
        let (fft_tx, receiver) = crossbeam_channel::bounded(3);

        runtime.spawn(start_sound_trigger_thread(receiver));
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
            SOUND_TRIGGER_THREAD.write().unwrap().reset();
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundTriggerHandle {
    uuid: Uuid,
    params: Arc<Mutex<SoundTriggerParams>>,
}

impl Drop for SoundTriggerHandle {
    fn drop(&mut self) {
        SOUND_TRIGGER_THREAD
            .write()
            .unwrap()
            .triggers
            .remove(&self.uuid);
    }
}

impl PartialEq for SoundTriggerHandle {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

impl SoundTriggerHandle {
    pub fn update_params_and_fetch_trigger(&self) -> Option<SoundTrigger> {
        SOUND_TRIGGER_THREAD
            .write()
            .unwrap()
            .triggers
            .get_mut(&self.uuid)
            .map(|trigger| {
                let mut trigger = trigger.lock().unwrap();
                trigger.params = *self.params.lock().unwrap();
                trigger.clone()
            })
    }
    pub fn level(&self) -> f32 {
        SOUND_TRIGGER_THREAD
            .write()
            .unwrap()
            .triggers
            .get_mut(&self.uuid)
            .map(|trigger| trigger.lock().unwrap().current_level)
            .unwrap_or(0.0)
    }
}

impl Hash for SoundTriggerHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.uuid.hash(state);
    }
}

pub struct SoundTriggeThread {
    triggers: HashMap<Uuid,Arc<Mutex<SoundTrigger>>>,
}

impl SoundTriggeThread {
    fn new() -> Self {
        Self {
            triggers: HashMap::new(),
        }
    }
    pub fn register_sound_trigger(&mut self, sound_trigger_params: SoundTriggerParams) -> SoundTriggerHandle {
        let uuid = Uuid::new_v4();
        let trigger = SoundTrigger::new(sound_trigger_params, SOUND_TRIGGER_SAMPLE_INTERVAL_MS as f32 / 1000.);
        self.triggers.insert(uuid, Arc::new(Mutex::new(trigger)));
        SoundTriggerHandle {
            uuid,
            params: Arc::new(Mutex::new(sound_trigger_params)),
        }
    }

    fn tick(&mut self, root_sample: [f32; FREQ_BINS]) {
        #[cfg(feature = "profiling")]
        puffin::profile_scope!("tick_sound_triggers");
        self.triggers
            .values_mut()
            .for_each(move |trigger| {
                let trigger = Arc::clone(trigger);
                let sample_copy = root_sample.clone();
                let mut trigger_guard = trigger.lock().unwrap();
                trigger_guard.tick(sample_copy);
            });
    }

    fn reset(&mut self) {
        self.triggers
            .values_mut()
            .for_each(move |trigger| {
                let trigger = Arc::clone(trigger);
                let mut trigger_guard = trigger.lock().unwrap();
                trigger_guard.reset();
            });
    }
}
pub async fn start_sound_trigger_thread(rx: Receiver<[f32; FREQ_BINS]>) {
    loop {
        {
            // give tokio the opportunity to break the sound trigger thread loop
            tokio::task::yield_now().await;
            #[cfg(feature = "profiling")]
            puffin::profile_scope!("SoundTriggerThread::waitForTrigger");
            let root_sample = match rx.recv_timeout(Duration::from_millis(10)) {
                Ok(root_sample) => root_sample,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => {
                    log::error!("Audio receiver disconnected, shutting down sound trigger thread");
                    break
                },
            };
            SOUND_TRIGGER_THREAD.write().unwrap().tick(root_sample);
        }
    }
}

// poll for audio device changes every second
pub fn audio_device_info_loop(continue_scan: &AtomicBool){
    #[cfg(feature = "profiling")]
    profiling::register_thread!("audio_device_info_loop");
    log::info!("Started scanning for audio devices...");
    while continue_scan.load(Relaxed)
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
        sleep(Duration::from_secs(1));
    }
    log::info!("Audio devices scanning stopped.");
}
