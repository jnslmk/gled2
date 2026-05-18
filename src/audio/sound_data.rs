use crate::audio::{
    SoundTriggerHandle,
    fft::FREQ_BINS,
    sound_trigger::{SoundTrigger, SoundTriggerParams},
};
use egui::mutex::Mutex;
use kanal::{Receiver, Sender, bounded};
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    sync::{Arc, atomic::AtomicUsize},
};
use tracing::trace;
use uuid::Uuid;

const SOUND_TRIGGER_SAMPLE_INTERVAL_MS: u64 = 10;

type Data = (HashMap<Uuid, SoundTrigger>, SoundTrigger);

static CURRENT: Lazy<Mutex<Data>> =
    Lazy::new(|| Mutex::new((HashMap::new(), SoundTrigger::default())));
static INSTANCE_ID: AtomicUsize = AtomicUsize::new(0);
static SENDERS: Lazy<Mutex<HashMap<usize, Sender<Data>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub struct SoundData {
    id: usize,
    receiver: Receiver<Data>,
    pub triggers: HashMap<Uuid, SoundTrigger>,
    pub default_trigger: SoundTrigger,
}

impl std::fmt::Debug for SoundData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SoundData").finish_non_exhaustive()
    }
}

impl Default for SoundData {
    fn default() -> Self {
        let (sender, receiver) = bounded(10);
        let id = INSTANCE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        SENDERS.lock().insert(id, sender);

        let (triggers, default_trigger) = CURRENT.lock().clone();

        Self {
            id,
            receiver,
            triggers,
            default_trigger,
        }
    }
}

impl Drop for SoundData {
    fn drop(&mut self) {
        SENDERS.lock().remove(&self.id);
    }
}

impl SoundData {
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
        Arc::new(SoundTriggerHandle { uuid })
    }

    pub fn fft_data_u8(&self) -> [u8; FREQ_BINS * 4] {
        let mut data = [0u8; FREQ_BINS * 4];

        for (i, sample) in self.default_trigger.spectrum.iter().enumerate() {
            let bytes = sample.to_le_bytes();
            data[i * 4..(i + 1) * 4].copy_from_slice(&bytes);
        }
        data
    }

    pub fn remove(&mut self, uuid: Uuid) {
        self.triggers.remove(&uuid);
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn tick(&mut self, fft_samples: [f32; FREQ_BINS]) {
        self.triggers.values_mut().for_each(move |trigger| {
            trigger.tick(fft_samples);
        });
        self.default_trigger.tick(fft_samples);
    }

    pub fn reset(&mut self) {
        self.triggers.values_mut().for_each(move |trigger| {
            trigger.reset();
        });
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn update(&mut self) {
        trace!("Updating sound trigger data (self.id = {})", self.id);
        while let Ok(Some((triggers, default_trigger))) = self.receiver.try_recv() {
            self.triggers = triggers;
            self.default_trigger = default_trigger;
        }
    }

    pub fn save(&self) {
        trace!("Saving sound trigger data (self.id = {})", self.id);
        for (id, sender) in SENDERS.lock().iter() {
            if id == &self.id {
                continue;
            }

            trace!("Sending sound trigger data to instance with id: {id}");
            sender
                .send((self.triggers.clone(), self.default_trigger.clone()))
                .expect("Could not send sound trigger data");
        }

        *CURRENT.lock() = (self.triggers.clone(), self.default_trigger.clone());
    }
}
