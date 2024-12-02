mod action;
mod asset;
mod asset_id;
mod collection;
mod git;

use collection::Collection;
use directories::BaseDirs;
use egui::mutex::Mutex;
use once_cell::sync::Lazy;
use std::{
    fmt::Debug,
    fs::remove_dir_all,
    path::PathBuf,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed},
};
use strum::Display;
use typemap::ShareDebugMap;
use uuid::Uuid;

use crate::app::PersistantState;

pub use self::{action::Action, asset::*, asset_id::AssetId};

pub static STORAGE_DIR: Lazy<PathBuf> = Lazy::new(|| {
    BaseDirs::new()
        .expect("Could not get base dirs")
        .data_dir()
        .join("gled2")
});
static WORKING: AtomicBool = AtomicBool::new(true);
static STATE: Lazy<Mutex<State>> = Lazy::new(|| Mutex::new(State::Loading(Loading::GitRepository)));
static BRANCHES: Lazy<Mutex<Option<Branches>>> = Lazy::new(|| Mutex::new(None));
static STAGED_FILES: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone)]
pub struct Branches {
    pub available: Vec<String>,
    pub current: String,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum State {
    Loading(Loading),
    Error(String),
    Loaded(ShareDebugMap),
}

impl State {
    pub fn set(self) {
        match &self {
            Self::Loaded(..) => println!("Storage is fully loaded."),
            state => println!("Storage state: {state:?}"),
        }
        *STATE.lock() = self;
    }
}

#[derive(Debug, Clone, Copy, Display)]
pub enum Loading {
    #[strum(serialize = "Git repository")]
    GitRepository,
    #[strum(serialize = "Git branches")]
    GitBranches,
    #[strum(serialize = "Git branch")]
    GitBranch,
    Animations,
    Curves,
    OutputDevices,
    Palettes,
    Projects,
    Scenes,
}

impl Loading {
    pub fn set(self) {
        State::Loading(self).set();
    }
}

pub fn working() -> bool {
    WORKING.load(Relaxed)
}

pub fn staged_files() -> usize {
    STAGED_FILES.load(Relaxed)
}

pub fn loaded() -> bool {
    matches!(*STATE.lock(), State::Loaded(..))
}

pub fn loading_state() -> Option<Loading> {
    if let State::Loading(loading) = *STATE.lock() {
        Some(loading)
    } else {
        None
    }
}

pub fn error_state() -> Option<String> {
    if let State::Error(err) = &*STATE.lock() {
        Some(err.clone())
    } else {
        None
    }
}

pub fn branches() -> Option<Branches> {
    BRANCHES.lock().clone()
}

pub fn start_thread() {
    let actions = action::init();

    std::thread::spawn(move || {
        let mut retry_wait = std::time::Duration::from_secs(0);
        loop {
            std::thread::sleep(retry_wait);
            retry_wait = std::time::Duration::from_secs(2);

            // Clear the queue
            while actions.try_recv().is_ok() {}

            Loading::GitRepository.set();
            let mut git = match git::Git::open(PersistantState::git_url()) {
                Ok(git) => git,
                Err(err) => {
                    let err: String = format!("Could not open git: {err}");
                    log::error!("{err}");
                    *STATE.lock() = State::Error(err);
                    continue;
                }
            };

            Action::LoadBranches.enqueue();
            Action::CountStagedFiles.enqueue();
            Action::LoadAssets.enqueue();

            while let Ok(action) = actions.recv() {
                WORKING.store(true, Relaxed);

                match action {
                    Action::Nuke => {
                        remove_dir_all(&*STORAGE_DIR).ok();
                        Action::Restart.enqueue();
                    }
                    Action::Restart => {
                        break;
                    }
                    Action::CountStagedFiles => match git.count_staged_files() {
                        Err(err) => {
                            State::Error(format!("Error counting staged files: {err}")).set();
                        }
                        Ok(count) => {
                            STAGED_FILES.store(count, Relaxed);
                        }
                    },
                    Action::Pull => {
                        if let Err(err) = git.pull() {
                            State::Error(format!("Error pulling: {err}")).set();
                        }
                    }
                    Action::CommitAndPush { message } => {
                        if let Err(err) = git.commit_and_push(&message) {
                            State::Error(format!("Error committing and pushing: {err}")).set();
                        }
                        Action::CountStagedFiles.enqueue();
                    }
                    Action::LoadBranches => {
                        BRANCHES.lock().take();

                        Loading::GitBranches.set();
                        let branches = match git.branches() {
                            Ok(branches) => branches,
                            Err(err) => {
                                let err: String = format!("Could not get branches: {err}");
                                log::error!("{err}");
                                State::Error(err).set();
                                continue;
                            }
                        };

                        Loading::GitBranch.set();
                        let current_branch = match git.current_branch() {
                            Ok(current_branch) => current_branch,
                            Err(err) => {
                                let err: String = format!("Could not get current_branch: {err}");
                                log::error!("{err}");
                                State::Error(err).set();
                                continue;
                            }
                        };

                        *BRANCHES.lock() = Some(Branches {
                            available: branches,
                            current: current_branch,
                        });
                    }
                    Action::LoadAssets => {
                        let mut collections = ShareDebugMap::custom();

                        Loading::Animations.set();
                        collections
                            .insert::<Collection<Animation>>(Collection::<Animation>::load());

                        Loading::Curves.set();
                        collections.insert::<Collection<Curve>>(Collection::<Curve>::load());

                        Loading::OutputDevices.set();
                        collections
                            .insert::<Collection<OutputDevice>>(Collection::<OutputDevice>::load());

                        Loading::Palettes.set();
                        collections.insert::<Collection<Palette>>(Collection::<Palette>::load());

                        Loading::Projects.set();
                        collections.insert::<Collection<Project>>(Collection::<Project>::load());

                        Loading::Scenes.set();
                        collections.insert::<Collection<Scene>>(Collection::<Scene>::load());

                        State::Loaded(collections).set();
                    }
                    Action::SwitchBranch(branch) => match git.switch_branch(&branch) {
                        Ok(_) => {
                            Action::Restart.enqueue();
                            Action::LoadBranches.enqueue();
                            Action::CountStagedFiles.enqueue();
                            Action::LoadAssets.enqueue();
                        }
                        Err(err) => {
                            State::Error(format!("Error switching branch: {err}")).set();
                        }
                    },
                    Action::SaveAsset {
                        dir_name,
                        uuid,
                        json,
                    } => {
                        if let Err(err) = git.write_asset(&asset_path(uuid, dir_name), json) {
                            log::error!("Could not write asset: {err:?}");
                        }

                        Action::CountStagedFiles.enqueue();
                    }
                    Action::DeleteAsset { uuid, dir_name } => {
                        if let Err(err) = git.delete_asset(&asset_path(uuid, dir_name)) {
                            log::error!("Could not delete asset: {err:?}");
                        }

                        Action::CountStagedFiles.enqueue();
                    }
                }

                WORKING.store(false, Relaxed);
            }
        }
    });
}

pub fn asset_path(id: Uuid, dir_name: &str) -> PathBuf {
    STORAGE_DIR.join(dir_name).join(format!("{}.json", id))
}
