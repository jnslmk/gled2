use crate::storage::SceneInstancePath;
use crossbeam_channel::{unbounded, Receiver, Sender};
use egui::mutex::Mutex;
use once_cell::sync::{Lazy, OnceCell};
use std::collections::HashSet;

static SENDER: OnceCell<Sender<MidiState>> = OnceCell::new();
static SENDERS: Lazy<Mutex<Vec<Sender<MidiState>>>> = Lazy::new(|| Mutex::new(Vec::new()));

#[derive(Debug, Clone)]
pub struct MidiState {
    pub blackout: bool,
    pub beat_flank: u8,
    pub active_scenes: HashSet<SceneInstancePath>,
}

impl MidiState {
    pub fn enqueue(self) {
        SENDER
            .get()
            .expect("Could not get sender")
            .send(self)
            .expect("Could not send state");
    }
}

pub fn start() {
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
