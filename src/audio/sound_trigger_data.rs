use crate::audio::{
    SoundTriggerHandle,
    fft::FREQ_BINS,
    sound_trigger::{SoundTrigger, SoundTriggerParams},
};
use egui::mutex::Mutex;
use kanal::{Receiver, Sender, bounded};
use log::trace;
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    sync::{Arc, atomic::AtomicUsize},
};
use uuid::Uuid;

const SOUND_TRIGGER_SAMPLE_INTERVAL_MS: u64 = 10;

static CURRENT: Lazy<Mutex<HashMap<Uuid, SoundTrigger>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static INSTANCE_ID: AtomicUsize = AtomicUsize::new(0);
static SENDERS: Lazy<Mutex<HashMap<usize, Sender<HashMap<Uuid, SoundTrigger>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub struct SoundTriggerData {
    id: usize,
    receiver: Receiver<HashMap<Uuid, SoundTrigger>>,
    pub triggers: HashMap<Uuid, SoundTrigger>,
}

impl Default for SoundTriggerData {
    fn default() -> Self {
        let (sender, receiver) = bounded(10);
        let id = INSTANCE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        SENDERS.lock().insert(id, sender);

        Self {
            id,
            receiver,
            triggers: CURRENT.lock().clone(),
        }
    }
}

impl Drop for SoundTriggerData {
    fn drop(&mut self) {
        SENDERS.lock().remove(&self.id);
    }
}

impl SoundTriggerData {
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

    pub fn remove(&mut self, uuid: Uuid) {
        self.triggers.remove(&uuid);
    }

    pub fn tick(&mut self, root_sample: [f32; FREQ_BINS]) {
        #[cfg(feature = "profiling")]
        puffin::profile_scope!("tick_sound_triggers");
        self.triggers.values_mut().for_each(move |trigger| {
            trigger.tick(root_sample);
        });
    }

    pub fn reset(&mut self) {
        self.triggers.values_mut().for_each(move |trigger| {
            trigger.reset();
        });
    }

    pub fn update(&mut self) {
        trace!("Updating sound trigger data (self.id = {})", self.id);
        while let Ok(Some(new_triggers)) = self.receiver.try_recv() {
            self.triggers = new_triggers;
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
                .send(self.triggers.clone())
                .expect("Could not send sound trigger data");
        }

        CURRENT.lock().clone_from(&self.triggers);
    }
}
