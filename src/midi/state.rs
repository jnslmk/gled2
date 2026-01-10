use crate::storage::asset::scene::color::SceneInstanceColor;
use crate::storage::asset::scene::grid::GridLocation;
use crossbeam_channel::{unbounded, Receiver, Sender};
use egui::mutex::Mutex;
use once_cell::sync::{Lazy, OnceCell};
use std::collections::{HashMap, HashSet};

static SENDER: OnceCell<Sender<MidiState>> = OnceCell::new();
static PREVIOUS_STATE: OnceCell<Mutex<MidiState>> = OnceCell::new();
static SENDERS: Lazy<Mutex<Vec<Sender<MidiState>>>> = Lazy::new(|| Mutex::new(Vec::new()));

#[derive(Debug, Clone, PartialEq)]
pub struct MidiState {
    pub blackout: bool,
    pub beat_flank: u8,
    pub active_scenes: HashSet<GridLocation>,
    pub flashed_scenes: HashSet<GridLocation>,
    pub available_scenes_grid: HashMap<GridLocation, SceneInstanceColor>,
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
