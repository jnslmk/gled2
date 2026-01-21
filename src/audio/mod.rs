pub mod adsr_editor;
pub mod reactive_signal;
pub mod state;

use crate::audio::reactive_signal::{AdsrParams, ReactiveSignal};
use crate::audio::state::fft_data;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::thread;
use std::thread::spawn;
use std::time::Duration;
use uuid::Uuid;

static REACTIVE_SIGNAL_THREAD: Lazy<RwLock<ReactiveSignalThread>> = Lazy::new(|| {RwLock::new(ReactiveSignalThread::new())});

static ADSR_SAMPLE_INTERVAL_MS: u64 = 10;


pub fn start_fft_thread() {
    spawn(state::start);
    spawn(start_reactive_sound_thread);
}

#[derive(Debug, Clone)]
pub struct ReactiveSignalHandle {
    uuid: Uuid,
    params: Arc<Mutex<AdsrParams>>,
}

impl PartialEq for ReactiveSignalHandle {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

impl ReactiveSignalHandle {
    pub fn update_params_and_fetch_signal(&self) -> Option<ReactiveSignal> {
        REACTIVE_SIGNAL_THREAD.write().unwrap().signals.get_mut(&self.uuid).map(|(_, signal)| {
            signal.params = self.params.lock().unwrap().clone();
            signal.clone()
        })
    }
}

impl Hash for ReactiveSignalHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.uuid.hash(state);
    }
}

pub struct ReactiveSignalThread {
    signals: HashMap<Uuid, (Weak<Mutex<AdsrParams>>, ReactiveSignal)>,
}

impl  ReactiveSignalThread {
    fn new() -> Self {
        Self { signals: HashMap::new() }
    }
    pub fn register_reactive_signal(&mut self) -> ReactiveSignalHandle {
        let uuid = Uuid::new_v4();
        let signal = ReactiveSignal::new(AdsrParams::default(), ADSR_SAMPLE_INTERVAL_MS as f32);
        let dead_mans_switch = Arc::new(Mutex::new(signal.params.clone()));
        self.signals.insert(
            uuid,
            (Arc::<Mutex<AdsrParams>>::downgrade(&dead_mans_switch), signal),
        );
        ReactiveSignalHandle{uuid, params: dead_mans_switch}
    }
}
    pub fn start_reactive_sound_thread() {
        spawn(||
        loop {
            let spectrum = fft_data();

            REACTIVE_SIGNAL_THREAD.write().unwrap().signals.values_mut().for_each(|(_, signal)| {
                signal.tick(&spectrum);
            });

            // Garbage collect references to ReactiveSignals which do not exist anymore
            REACTIVE_SIGNAL_THREAD.write().unwrap().signals
                .retain(|_, (dead_mans_switch, _)| dead_mans_switch.upgrade().is_some());

            // this delay needs to be long enough to allow the ui thread to copy data in time
            // this may be suboptimal
            thread::sleep(Duration::from_millis(ADSR_SAMPLE_INTERVAL_MS));
        });
    }