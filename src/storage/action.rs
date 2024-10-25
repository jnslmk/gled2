use super::{tree::Asset, Palette, Project, Scene};
use once_cell::sync::OnceCell;
use std::sync::mpsc::{Receiver, Sender};

static SENDER: OnceCell<Sender<Action>> = OnceCell::new();

pub fn init() -> Receiver<Action> {
    let (tx, rx) = std::sync::mpsc::channel();
    SENDER.set(tx).expect("Could not set ACTION_SENDER");
    rx
}

pub enum Action {
    Update,
    SwitchBranch(String),
    SavePalette { palette: Asset<Palette> },
    SaveProject { project: Asset<Project> },
    SaveScene { scene: Asset<Scene> },
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
