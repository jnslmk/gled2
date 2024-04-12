mod storage;

use once_cell::sync::OnceCell;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::RwLock;

pub static STATE: RwLock<State> = RwLock::new(State::Updating);
static ACTION_SENDER: OnceCell<Sender<Action>> = OnceCell::new();

#[derive(Debug, Default)]
pub enum State {
    #[default]
    Updating,
    Error(String),
    Opened {
        branches: Vec<String>,
        current_branch: String,
        folder: PathBuf,
    },
}

pub enum Action {
    Update,
    SwitchBranch(String),
    SaveFile {
        path: PathBuf,
        contents: Vec<u8>,
        message: String,
    },
}

impl Action {
    pub fn send(self) {
        ACTION_SENDER
            .get()
            .expect("Could not get ACTION_SENDER")
            .send(self)
            .expect("Could not send action");
    }
}

pub fn start_thread() {
    let (tx, rx) = std::sync::mpsc::channel();
    ACTION_SENDER.set(tx).expect("Could not set ACTION_SENDER");

    std::thread::spawn(move || {
        let mut retry_wait = std::time::Duration::from_secs(0);
        loop {
            *STATE.write().unwrap() = State::Updating;

            std::thread::sleep(retry_wait);
            retry_wait = std::time::Duration::from_secs(2);

            let mut storage =
                match storage::Storage::open("git@git.freshx.de:rene/gled2_assets.git".to_string())
                {
                    Ok(storage) => storage,
                    Err(err) => {
                        *STATE.write().unwrap() =
                            State::Error(format!("Could not open storage: {err}"));
                        continue;
                    }
                };

            let branches = match storage.branches() {
                Ok(branches) => branches,
                Err(err) => {
                    *STATE.write().unwrap() =
                        State::Error(format!("Could not get branches: {err}"));
                    continue;
                }
            };

            let current_branch = match storage.current_branch() {
                Ok(current_branch) => current_branch,
                Err(err) => {
                    *STATE.write().unwrap() =
                        State::Error(format!("Could not get current_branch: {err}"));
                    continue;
                }
            };

            let folder = storage.folder().to_owned();

            *STATE.write().unwrap() = State::Opened {
                branches: branches.clone(),
                current_branch,
                folder: folder.clone(),
            };

            while let Ok(action) = rx.recv() {
                *STATE.write().unwrap() = State::Updating;

                match action {
                    Action::Update => break,
                    Action::SwitchBranch(branch) => match storage.switch_branch(&branch) {
                        Ok(_) => {
                            *STATE.write().unwrap() = State::Opened {
                                branches: branches.clone(),
                                current_branch: branch,
                                folder: folder.clone(),
                            };
                        }
                        Err(err) => {
                            *STATE.write().unwrap() =
                                State::Error(format!("Error switching branch: {err}"));
                            break;
                        }
                    },
                    Action::SaveFile {
                        path,
                        contents,
                        message,
                    } => {
                        //TODO: Mkdirp, save, saveandPush
                    }
                }
            }
        }
    });
}
