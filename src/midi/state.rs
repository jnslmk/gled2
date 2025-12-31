use crate::storage::asset::{
    project::scene_instance_path::SceneInstancePathIndex, scene::color::SceneInstanceColor,
};
use crossbeam_channel::{Receiver, Sender, unbounded};
use egui::mutex::Mutex;
use once_cell::sync::{Lazy, OnceCell};
use std::collections::HashSet;

static SENDER: OnceCell<Sender<MidiState>> = OnceCell::new();
static PREVIOUS_STATE: OnceCell<Mutex<MidiState>> = OnceCell::new();
static SENDERS: Lazy<Mutex<Vec<Sender<MidiState>>>> = Lazy::new(|| Mutex::new(Vec::new()));

#[derive(Debug, Clone, PartialEq)]
pub struct MidiState {
    pub blackout: bool,
    pub beat_flank: u8,
    pub active_scenes: HashSet<SceneInstancePathIndex>,
    pub flashed_scenes: HashSet<SceneInstancePathIndex>,
    pub available_scenes_grid: Vec<SceneInstanceColor>,
    pub available_scenes_quick: Vec<SceneInstanceColor>,
    pub selected_scene_opacity: f32,
}
impl Eq for MidiState {}

impl MidiState {
    pub fn enqueue(self) {
        let mut previous = PREVIOUS_STATE
            .get_or_init(|| Mutex::new(self.clone()))
            .lock();
        if *previous == self {
            return;
        }
        *previous = self.clone();
        SENDER
            .get()
            .expect("Could not get sender")
            .send(self)
            .expect("Could not send state");
    }
}

pub fn start() {
    #[cfg(feature = "profiling")]
    profiling::register_thread!("midi:state");

    let (sender, receiver) = unbounded();
    SENDER.set(sender).expect("Could not set sender");

    loop {
        let state = receiver.recv().expect("Could not receive state");
        SENDERS
            .lock()
            .retain(|sender| sender.send(state.clone()).is_ok());
    }
}

pub fn new_receiver() -> Receiver<MidiState> {
    let (sender, receiver) = unbounded();
    SENDERS.lock().push(sender);
    receiver
}
