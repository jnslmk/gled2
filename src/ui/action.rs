use crate::{
    app::Svg,
    storage::{Animation, AssetId, Project},
};
use egui::ViewportId;
use once_cell::sync::OnceCell;
use std::sync::mpsc::{Receiver, Sender};

static ACTION_SENDER: OnceCell<Sender<Action>> = OnceCell::new();

pub enum Action {
    SetProject(AssetId<Project>),
    DeleteSelectedSceneInstance,
    CloneSelectedSceneInstance,
    InitGPU,
    SendPositions,
    ReloadShaderCode(AssetId<Animation>),
    CloseWindow(ViewportId),
    SetSvg(Option<Svg>),
}

impl Action {
    pub fn init_queue() -> Receiver<Action> {
        let (sender, receiver) = std::sync::mpsc::channel();
        ACTION_SENDER.set(sender).unwrap();
        receiver
    }

    pub fn enqueue(self) {
        ACTION_SENDER
            .get()
            .expect("Action sender not initialized")
            .send(self)
            .expect("Action receiver dropped");
    }
}
