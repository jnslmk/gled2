mod storage;

use once_cell::sync::OnceCell;
use rayon::prelude::*;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::RwLock;

pub static STATE: RwLock<State> = RwLock::new(State::Loading(0.0));
static ACTION_SENDER: OnceCell<Sender<Action>> = OnceCell::new();

#[derive(Debug)]
pub enum State {
    Loading(f32),
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
            *STATE.write().unwrap() = State::Loading(0.0);

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

            *STATE.write().unwrap() = State::Loading(0.1);

            let branches = match storage.branches() {
                Ok(branches) => branches,
                Err(err) => {
                    *STATE.write().unwrap() =
                        State::Error(format!("Could not get branches: {err}"));
                    continue;
                }
            };

            *STATE.write().unwrap() = State::Loading(0.15);

            let current_branch = match storage.current_branch() {
                Ok(current_branch) => current_branch,
                Err(err) => {
                    *STATE.write().unwrap() =
                        State::Error(format!("Could not get current_branch: {err}"));
                    continue;
                }
            };

            *STATE.write().unwrap() = State::Loading(0.2);

            let folder = storage.folder().to_owned();

            //TODO: Recursive find files in folders and parse
            //folder.join("palette")

            *STATE.write().unwrap() = State::Opened {
                branches: branches.clone(),
                current_branch,
                folder: folder.clone(),
            };

            while let Ok(action) = rx.recv() {
                *STATE.write().unwrap() = State::Loading(0.0);

                match action {
                    Action::Update => break,
                    Action::SwitchBranch(branch) => match storage.switch_branch(&branch) {
                        Ok(_) => {
                            *STATE.write().unwrap() = State::Opened {
                                branches: branches.clone(),
                                current_branch: branch,
                                folder: folder.clone(),
                            };
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
                        contents,
                        message,
                    } => {
                        //TODO: Mkdirp, save, saveandPush and also save in state
                    }
                }
            }
        }
    });
}

trait AssetTrait: Serialize + Send + DeserializeOwned + Debug {}

pub struct Directory<T: AssetTrait> {
    path: PathBuf,
    pub assets: HashMap<String, Asset<T>>,
    pub subdirectories: HashMap<String, Directory<T>>,
}

impl<T: AssetTrait> Directory<T> {
    pub fn load(path: PathBuf) -> Result<Self, std::io::Error> {
        let paths = path.read_dir()?;

        enum Entry<T: AssetTrait> {
            Directory(Directory<T>),
            Asset(Asset<T>),
        }

        let mut subdirectories = HashMap::new();
        let mut assets = HashMap::new();

        let entries = paths
            .par_bridge()
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let file_name = entry.file_name().into_string().ok()?;
                let file_type = entry.file_type().ok()?;

                if file_type.is_dir() {
                    return Some((
                        file_name,
                        Entry::Directory(Directory::load(entry.path()).ok()?),
                    ));
                }

                if file_type.is_file() {
                    let file = std::fs::File::open(entry.path()).ok()?;
                    let data = serde_json::from_reader(file).ok()?;

                    return Some((
                        file_name,
                        Entry::Asset(Asset {
                            path: entry.path(),
                            data,
                        }),
                    ));
                }

                None
            })
            .collect::<Vec<_>>();

        for (file_name, entry) in entries {
            match entry {
                Entry::Directory(directory) => {
                    subdirectories.insert(file_name, directory);
                }
                Entry::Asset(asset) => {
                    assets.insert(file_name, asset);
                }
            }
        }

        Ok(Self {
            path,
            assets,
            subdirectories,
        })
    }
}

pub struct Asset<T: AssetTrait> {
    path: PathBuf,
    pub data: T,
}

impl<T: AssetTrait> Asset<T> {
    pub fn update(&self, data: T) {
        let contents = serde_json::to_vec(&data).expect("Could not serialize data");

        Action::SaveFile {
            path: self.path.clone(),
            contents,
            message: format!("Update asset {}", self.path.display()),
        }
        .send();
    }
}
