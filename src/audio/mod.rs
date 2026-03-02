pub mod device_id_serde;
pub mod fft;
pub mod sound_trigger;
pub mod sound_trigger_data;
pub mod sound_trigger_editor;

use crate::audio::{
    fft::{AudioSource, FREQ_BINS},
    sound_trigger::{SoundTrigger, SoundTriggerParams},
    sound_trigger_data::SoundTriggerData,
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
    hash::Hash,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering::Relaxed},
    },
    thread::sleep,
    time::Duration,
};
use uuid::Uuid;

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
        let mut data = SoundTriggerData::default();
        data.reset();
        data.save();
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SoundTriggerHandle {
    uuid: Uuid,
}

impl Drop for SoundTriggerHandle {
    fn drop(&mut self) {
        let mut data = SoundTriggerData::default();
        data.remove(self.uuid);
        data.save();
    }
}

impl PartialEq for SoundTriggerHandle {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

impl SoundTriggerHandle {
    pub fn get_sound_trigger<'a>(
        &'a self,
        sound_trigger_data: &'a SoundTriggerData,
    ) -> Option<&'a SoundTrigger> {
        sound_trigger_data.triggers.get(&self.uuid)
    }

    pub fn get_sound_trigger_mut<'a>(
        &'a self,
        sound_trigger_data: &'a mut SoundTriggerData,
    ) -> Option<&'a mut SoundTrigger> {
        sound_trigger_data.triggers.get_mut(&self.uuid)
    }

    pub fn get_params<'a>(
        &'a self,
        sound_trigger_data: &'a SoundTriggerData,
    ) -> Option<&'a SoundTriggerParams> {
        Some(&self.get_sound_trigger(sound_trigger_data)?.params)
    }

    pub fn get_params_mut<'a>(
        &'a self,
        sound_trigger_data: &'a mut SoundTriggerData,
    ) -> Option<&'a mut SoundTriggerParams> {
        Some(&mut self.get_sound_trigger_mut(sound_trigger_data)?.params)
    }

    pub fn level(&self, sound_trigger_data: &SoundTriggerData) -> f32 {
        sound_trigger_data
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

pub fn start_sound_trigger_thread() -> Sender<[f32; FREQ_BINS]> {
    let (tx, rx) = kanal::bounded(0);

    std::thread::Builder::new()
        .name("gled:audio:trigger".to_string())
        .spawn(move || {
            #[cfg(feature = "profiling")]
            profiling::register_thread!("SoundTriggerThread");
            log::info!("Sound trigger thread started");
            let mut data = SoundTriggerData::default();

            while let Ok(root_sample) = rx.recv() {
                data.update();
                data.tick(root_sample);
                data.save();
            }
        })
        .expect("Could not spawn sound trigger thread");

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
