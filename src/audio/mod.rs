pub mod adsr_editor;
mod reactive_signal;
pub mod state;

use crate::audio::reactive_signal::ReactiveSignal;
use crate::audio::state::fft_data;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, RwLock, Weak};
use std::thread;
use std::thread::spawn;
use std::time::Duration;
use uuid::Uuid;

static REACTIVE_SIGNALS: Lazy<Arc<RwLock<HashMap<Uuid, Weak<RwLock<ReactiveSignal>>>>>> =
    Lazy::new(|| {
        Arc::new(RwLock::new(
            HashMap::<Uuid, Weak<RwLock<ReactiveSignal>>>::new(),
        ))
    });

static ADSR_SAMPLE_INTERVAL_MS: u64 = 10;


pub fn start_fft_thread() {
    spawn(state::start);
    spawn(start_reactive_sound_thread);
}

pub fn register_reactive_signal(signal: ReactiveSignal) -> Arc<RwLock<ReactiveSignal>> {
    let uuid = Uuid::new_v4();
    let dead_mans_switch = Arc::new(RwLock::new(signal));
    REACTIVE_SIGNALS.write().unwrap().insert(
        uuid,
        Arc::<RwLock<ReactiveSignal>>::downgrade(&dead_mans_switch),
    );
    dead_mans_switch
}

pub fn start_reactive_sound_thread() {
    loop {
        let spectrum = fft_data();

        REACTIVE_SIGNALS.read().unwrap().values().for_each(|signal| {
            if let Some(signal) = signal.upgrade() {
                signal.write().unwrap().tick(&spectrum);
            }
        });

        // Garbage collect references to ReactiveSignals which do not exist anymore
        REACTIVE_SIGNALS.write().unwrap()
            .retain(|_, dead_mans_switch| dead_mans_switch.upgrade().is_some());

        // this delay needs to be long enough to allow the ui thread to copy data in time
        // this may be suboptimal
        thread::sleep(Duration::from_millis(ADSR_SAMPLE_INTERVAL_MS));
    }
}
