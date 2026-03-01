pub mod device_id_serde;
pub mod fft;
pub mod sound_trigger;
pub mod sound_trigger_editor;

use crate::audio::{
    fft::{AudioSource, FREQ_BINS},
    sound_trigger::{SoundTrigger, SoundTriggerParams},
};
use cpal::DeviceDirection::{Duplex, Input};
use cpal::{
    DeviceDescription, DeviceId,
    traits::{DeviceTrait, HostTrait},
};
use kanal::Sender;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    hash::Hash,
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, Ordering::Relaxed},
    },
    thread::sleep,
    time::Duration,
};
use uuid::Uuid;

pub static SOUND_TRIGGER_THREAD_DATA: Lazy<RwLock<SoundTriggerThreadData>> =
    Lazy::new(|| RwLock::new(SoundTriggerThreadData::new()));
static SOUND_TRIGGER_SAMPLE_INTERVAL_MS: u64 = 10;
pub static AUDIO_DEVICES: Lazy<Mutex<Vec<(DeviceId, DeviceDescription)>>> =
    Lazy::new(|| Mutex::new(Vec::new()));

#[derive(Debug, Default)]
pub struct AudioPool {
    pub selected_device: Option<DeviceId>,
    pub audio_source: Option<AudioSource>,
}

impl AudioPool {
    pub fn restart_fft(&mut self) {
        self.audio_source = self.selected_device.clone().map(|device_id| {
            log::info!("Restarting FFT with device: {:?}", device_id);
            let fft_tx = start_sound_trigger_thread();
            AudioSource::start(device_id, fft_tx)
        });
        SOUND_TRIGGER_THREAD_DATA.write().unwrap().reset();
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SoundTriggerHandle {
    uuid: Uuid,
    params: Mutex<SoundTriggerParams>,
}

impl Drop for SoundTriggerHandle {
    fn drop(&mut self) {
        SOUND_TRIGGER_THREAD_DATA
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
        SOUND_TRIGGER_THREAD_DATA
            .read()
            .unwrap()
            .triggers
            .get(&self.uuid)
            .cloned()
    }
    pub fn level(&self) -> f32 {
        SOUND_TRIGGER_THREAD_DATA
            .read()
            .unwrap()
            .triggers
            .get(&self.uuid)
            .map(|trigger| trigger.current_level)
            .unwrap_or(0.0)
    }
}

impl Hash for SoundTriggerHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.uuid.hash(state);
    }
}

pub struct SoundTriggerThreadData {
    triggers: HashMap<Uuid, SoundTrigger>,
}

impl SoundTriggerThreadData {
    fn new() -> Self {
        Self {
            triggers: HashMap::new(),
        }
    }
    pub fn register_sound_trigger(
        &mut self,
        sound_trigger_params: SoundTriggerParams,
    ) -> Arc<SoundTriggerHandle> {
        let uuid = Uuid::new_v4();
        let trigger = SoundTrigger::new(
            sound_trigger_params,
            SOUND_TRIGGER_SAMPLE_INTERVAL_MS as f32 / 1000.,
        );
        self.triggers.insert(uuid, trigger);
        Arc::new(SoundTriggerHandle {
            uuid,
            params: Mutex::new(sound_trigger_params),
        })
    }

    fn tick(&mut self, root_sample: [f32; FREQ_BINS]) {
        #[cfg(feature = "profiling")]
        puffin::profile_scope!("tick_sound_triggers");
        self.triggers.values_mut().for_each(move |trigger| {
            trigger.tick(root_sample);
        });
    }

    fn reset(&mut self) {
        self.triggers.values_mut().for_each(move |trigger| {
            trigger.reset();
        });
    }
}
pub fn start_sound_trigger_thread() -> Sender<[f32; FREQ_BINS]> {
    let (tx, rx) = kanal::bounded(0);

    std::thread::spawn(move || {
        #[cfg(feature = "profiling")]
        profiling::register_thread!("SoundTriggerThread");
        log::info!("Sound trigger thread started");
        while let Ok(root_sample) = rx.recv() {
            SOUND_TRIGGER_THREAD_DATA.write().unwrap().tick(root_sample);
        }
    });

    tx
}

// poll for audio device changes every second
pub fn audio_device_info_loop(continue_scan: &AtomicBool) {
    #[cfg(feature = "profiling")]
    profiling::register_thread!("audio_device_info_loop");
    log::info!("Started scanning for audio devices...");
    while continue_scan.load(Relaxed) {
        #[cfg(feature = "profiling")]
        puffin::profile_scope!("audio_device_info_loop");

        let host = cpal::default_host();
        let devices = host.input_devices().expect("Failed to get audio devices");
        let device_map = Vec::from_iter(
            devices
                .map(|device| {
                    (
                        device.id().expect("Failed to get audio device id"),
                        device
                            .description()
                            .expect("Failed to get audio device description"),
                    )
                })
                .filter(|(_, desc)| desc.direction() == Input || desc.direction() == Duplex),
        );

        *AUDIO_DEVICES.lock().unwrap() = device_map;
        sleep(Duration::from_secs(1));
    }
    log::info!("Audio devices scanning stopped.");
}
