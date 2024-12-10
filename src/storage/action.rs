use std::sync::{
    mpsc::{Receiver, Sender},
    OnceLock,
};
use uuid::Uuid;

static SENDER: OnceLock<Sender<Action>> = OnceLock::new();

pub fn init() -> Receiver<Action> {
    let (tx, rx) = std::sync::mpsc::channel();
    SENDER.set(tx).expect("Could not set ACTION_SENDER");
    rx
}

#[allow(dead_code)]
pub enum Action {
    /// Nuke the storage folder and restart from scratch
    Nuke,
    /// Restart the storage system
    Restart,
    LoadBranches,
    LoadAssets,
    SwitchBranch(String),
    SaveAsset {
        dir_name: &'static str,
        uuid: Uuid,
        json: String,
    },
    DeleteAsset {
        dir_name: &'static str,
        uuid: Uuid,
    },
    Pull,
    Push,
    CountStagedFiles,
    Commit {
        message: String,
    },
}

impl Action {
    pub fn enqueue(self) {
        SENDER
            .get()
            .expect("Could not get ACTION_SENDER")
            .send(self)
            .expect("Could not send action");
    }
}
