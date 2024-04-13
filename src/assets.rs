mod storage;
mod tree;

use self::tree::Tree;
use crate::{effect::Effect, scene::Scene};
use once_cell::sync::OnceCell;
use std::{
    fmt::Debug,
    path::PathBuf,
    sync::{mpsc::Sender, RwLock},
};

pub static STATE: RwLock<State> = RwLock::new(State::Loading(0.0));
static ACTION_SENDER: OnceCell<Sender<Action>> = OnceCell::new();

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum State {
    Loading(f32),
    Error(String),
    Opened {
        branches: Vec<String>,
        current_branch: String,
        folder: PathBuf,
        effects: Tree<Effect>,
        scenes: Tree<Scene>,
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
            *STATE.write().unwrap() = State::Loading(0.0);

            std::thread::sleep(retry_wait);
            retry_wait = std::time::Duration::from_secs(2);

            let storage =
                match storage::Storage::open("git@git.freshx.de:rene/gled2_assets.git".to_string())
                {
                    Ok(storage) => storage,
                    Err(err) => {
                        *STATE.write().unwrap() =
                            State::Error(format!("Could not open storage: {err}"));
                        continue;
                    }
                };

            *STATE.write().unwrap() = State::Loading(0.2);

            let branches = match storage.branches() {
                Ok(branches) => branches,
                Err(err) => {
                    *STATE.write().unwrap() =
                        State::Error(format!("Could not get branches: {err}"));
                    continue;
                }
            };

            *STATE.write().unwrap() = State::Loading(0.3);

            let current_branch = match storage.current_branch() {
                Ok(current_branch) => current_branch,
                Err(err) => {
                    *STATE.write().unwrap() =
                        State::Error(format!("Could not get current_branch: {err}"));
                    continue;
                }
            };

            *STATE.write().unwrap() = State::Loading(0.4);

            let folder = storage.folder().to_owned();

            //TODO: Implement
            //let palettes = Tree::load(folder.join("palettes"));
            *STATE.write().unwrap() = State::Loading(0.6);
            let effects = Tree::<Effect>::load(folder.join("effects"));
            *STATE.write().unwrap() = State::Loading(0.8);
            let scenes = Tree::<Scene>::load(folder.join("scenes"));
            *STATE.write().unwrap() = State::Loading(1.0);

            *STATE.write().unwrap() = State::Opened {
                branches,
                current_branch,
                folder: folder.clone(),
                effects,
                scenes,
            };

            dbg!(STATE.read());

            while let Ok(action) = rx.recv() {
                *STATE.write().unwrap() = State::Loading(0.0);

                match action {
                    Action::Update => break,
                    Action::SwitchBranch(branch) => match storage.switch_branch(&branch) {
                        Ok(_) => {
                            // needs to reload all assets
                            break;
                        }
                        Err(err) => {
                            *STATE.write().unwrap() =
                                State::Error(format!("Error switching branch: {err}"));
                            break;
                        }
                    },
                    Action::SaveFile {
                        path,
                        contents: _contents,
                        message: _message,
                    } => {
                        let Ok(path) = path.strip_prefix(&folder) else {
                            continue;
                        };
                        for _component in path.components() {}

                        //TODO: Mkdirp, save, saveandPush and also save in state
                    }
                }
            }
        }
    });
}
