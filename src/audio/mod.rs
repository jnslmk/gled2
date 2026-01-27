pub mod adsr_editor;
pub mod reactive_signal;
pub mod state;

use crate::audio::reactive_signal::{AdsrParams, ReactiveSignal};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::Ordering::Relaxed;
use std::thread;
use std::thread::spawn;
use std::time::Duration;
use uuid::Uuid;
use crate::audio::state::{FFT_DATA, RMS_INDEX};

pub static REACTIVE_SIGNAL_THREAD: Lazy<RwLock<ReactiveSignalThread>> =
    Lazy::new(|| RwLock::new(ReactiveSignalThread::new()));

static ADSR_SAMPLE_INTERVAL_MS: u64 = 10;

pub fn start_fft_thread() {
    spawn(state::start);
    spawn(start_reactive_sound_thread);
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
pub fn start_reactive_sound_thread() {
    #[cfg(feature = "profiling")]
    profiling::register_thread!("audio");
    loop {
        {
            #[cfg(feature = "profiling")]
            puffin::profile_scope!("ReactiveSignalThread::tick");
            let current_rms_sample = FFT_DATA[RMS_INDEX.load(Relaxed)].lock().clone();
            let current_rms_index = RMS_INDEX.load(Relaxed);
            REACTIVE_SIGNAL_THREAD
                .write()
                .unwrap()
                .signals
                .values_mut()
                .for_each(|signal| {
                    signal.tick(current_rms_sample, current_rms_index);
                });
        }
        // this delay needs to be long enough to allow the ui thread to copy data in time
        // this may be suboptimal
        thread::sleep(Duration::from_millis(ADSR_SAMPLE_INTERVAL_MS));
    }
}
