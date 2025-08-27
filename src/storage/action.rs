use std::sync::{
    OnceLock,
    mpsc::{Receiver, Sender},
};
use uuid::Uuid;

static SENDER: OnceLock<Sender<StorageAction>> = OnceLock::new();

pub fn init() -> Receiver<StorageAction> {
    let (tx, rx) = std::sync::mpsc::channel();
    SENDER.set(tx).expect("Could not set ACTION_SENDER");
    rx
}

#[derive(Debug)]
pub enum StorageAction {
    /// Nuke the storage folder and restart from scratch
    Nuke,
    /// Restart the storage system
    Restart,
    /// Stop the storage system
    Stop,
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

impl StorageAction {
    pub fn enqueue(self) {
        SENDER
            .get()
            .expect("Could not get ACTION_SENDER")
            .send(self)
            .expect("Could not send action");
    }
}
