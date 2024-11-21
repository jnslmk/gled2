use crate::storage::{Animation, AssetId};
use egui::mutex::Mutex;
use once_cell::sync::Lazy;
use std::collections::VecDeque;

static ACTION_QUEUE: Lazy<Mutex<VecDeque<Action>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

pub enum Action {
    DeleteSelectedSceneInstance,
    CloneSelectedSceneInstance,
    InitGPU,
    SendPositions,
    ReloadShaderCode(AssetId<Animation>),
}

impl Action {
    pub fn enqueue(self) {
        ACTION_QUEUE.lock().push_back(self);
    }

    pub fn dequeue() -> Option<Self> {
        ACTION_QUEUE.lock().pop_front()
    }
}
