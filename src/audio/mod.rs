pub mod adsr_editor;
pub mod reactive_signal;
pub mod fft;

use crate::audio::fft::{FFT_DATA, RMS_INDEX};
use crate::audio::reactive_signal::{AdsrParams, ReactiveSignal};
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{DeviceDescription, DeviceId};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash};
use std::sync::atomic::Ordering::Relaxed;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::thread::spawn;
use std::time::Duration;
use uuid::Uuid;

pub static REACTIVE_SIGNAL_THREAD: Lazy<RwLock<ReactiveSignalThread>> =
    Lazy::new(|| RwLock::new(ReactiveSignalThread::new()));
static ADSR_SAMPLE_INTERVAL_MS: u64 = 10;
pub static FFT_THREAD: Lazy<Mutex<FFTThread>> = Lazy::new(|| Mutex::new(FFTThread::init()));
pub static AUDIO_DEVICES: Lazy<Mutex<Vec<(DeviceId, DeviceDescription)>>> = Lazy::new(|| Mutex::new(Vec::new()));


pub fn start_fft_thread() {
    let host = cpal::default_host();
    let default_device = host.default_input_device();
    if let Some(default_device) = default_device {
        {
            let mut fft_thread = FFT_THREAD.lock().unwrap();
            fft_thread.selected_device = default_device.id().ok();
            fft_thread.restart_fft(None);
        }
    }
    spawn(start_reactive_sound_thread);
}

#[derive(Debug)]
pub struct FFTThread{
    handle: Option<(thread::JoinHandle<()>, Sender<()>)>,
    pub selected_device: Option<DeviceId>,
}

impl FFTThread{
    pub fn init() -> FFTThread {
        spawn(audio_device_info_loop);
        Self{handle: None, selected_device: None}
    }
    pub fn restart_fft(&mut self, device_id: Option<DeviceId>){
        if let Some(handle) = self.handle.take() {
            // wait for the thread to stop
            handle.1.send(()).unwrap();
            handle.0.join().unwrap();
        }

        if let Some(device_id) = device_id {
            let (tx, rx) = channel::<()>();
            let handle = spawn(|| fft::start(rx, device_id));
            self.handle = Some((handle, tx));
        }
        else {
            self.handle = None
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

// poll for audio device changes every second
pub fn audio_device_info_loop(){
    #[cfg(feature = "profiling")]
    profiling::register_thread!("audio_device_info_loop");
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

            eprintln!("{:?}", device_map);
            *AUDIO_DEVICES.lock().unwrap() = device_map;
        }
        thread::sleep(Duration::from_millis(1000));
    }
}
