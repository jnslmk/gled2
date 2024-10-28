use super::{asset::Asset, AssetId, Palette, Project, Scene};
use std::sync::{
    mpsc::{Receiver, Sender},
    OnceLock,
};

static SENDER: OnceLock<Sender<Action>> = OnceLock::new();

pub fn init() -> Receiver<Action> {
    let (tx, rx) = std::sync::mpsc::channel();
    SENDER.set(tx).expect("Could not set ACTION_SENDER");
    rx
}

pub enum Action {
    Update,
    SwitchBranch(String),
    SavePalette { palette: Asset<Palette> },
    DeletePalette { id: AssetId<Palette> },
    SaveProject { project: Asset<Project> },
    DeleteProject { id: AssetId<Project> },
    SaveScene { scene: Asset<Scene> },
    DeleteScene { id: AssetId<Scene> },
    CommitAndPush { message: String },
}

impl Action {
    pub fn send(self) {
        SENDER
            .get()
            .expect("Could not get ACTION_SENDER")
            .send(self)
            .expect("Could not send action");
    }
}
