use crate::{
    app::Svg,
    storage::{Animation, AssetId, Project},
};
use egui::ViewportId;
use once_cell::sync::OnceCell;
use std::sync::mpsc::{Receiver, Sender};

static ACTION_SENDER: OnceCell<Sender<UiAction>> = OnceCell::new();

pub enum UiAction {
    SetProject(AssetId<Project>),
    DeleteSelectedSceneInstance,
    CloneSelectedSceneInstance,
    InitGPU,
    SendPositions,
    ReloadShaderCode(AssetId<Animation>),
    CloseWindow(ViewportId),
    SetSvg(Option<Svg>),
    OpenGitConfigWindow,
}

impl UiAction {
    pub fn init_queue() -> Receiver<UiAction> {
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
