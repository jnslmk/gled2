use crate::storage::asset::{project::Project, scene::grid::GridLocation};
use egui::mutex::Mutex;
use kanal::{Receiver, Sender, unbounded};
use once_cell::sync::{Lazy, OnceCell};

static SENDER: OnceCell<Sender<ProjectState>> = OnceCell::new();
static SUBSCRIBERS: Lazy<Mutex<Vec<Sender<ProjectState>>>> = Lazy::new(|| Mutex::new(Vec::new()));

#[derive(Debug, Clone)]
pub struct ProjectState {
    pub project: Option<Project>,
    pub selected_scene_instance: GridLocation,
    pub blackout: bool,
    pub beats_per_minute: f32,
    pub beat_progression: f32,
}

impl ProjectState {
    pub fn enqueue(self) {
        SENDER
            .get()
            .expect("Could not get project state sender")
            .send(self)
            .expect("Could not send project state");
    }
}

pub fn init() {
    let (sender, receiver) = unbounded();
    SENDER.set(sender).expect("Could not set project state sender");

    std::thread::spawn(move || {
        #[cfg(feature = "profiling")]
        profiling::register_thread!("output:state");

        for state in receiver {
            SUBSCRIBERS
                .lock()
                .retain(|subscriber| subscriber.send(state.clone()).is_ok());
        }
    });
}

pub fn new_receiver() -> Receiver<ProjectState> {
    let (sender, receiver) = unbounded();
    SUBSCRIBERS.lock().push(sender);
    receiver
}
