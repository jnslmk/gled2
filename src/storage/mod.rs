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
    thread::sleep,
    time::Duration,
};
use strum::Display;
use typemap::ShareDebugMap;
use uuid::Uuid;

use crate::app::PersistantState;

pub use self::{action::Action, asset::*, asset_id::AssetId};
pub use git::{GitCredentials, SSH_KEY_PASSPHRASE_ENTRY};

pub static STORAGE_DIR: Lazy<PathBuf> = Lazy::new(|| {
    BaseDirs::new()
        .expect("Could not get base dirs")
        .data_dir()
        .join("gled2")
});
static WORKING: AtomicBool = AtomicBool::new(true);
static ERROR: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));
static LOADING: Lazy<Mutex<Option<Loading>>> = Lazy::new(|| Mutex::new(None));
static COLLECTIONS: Lazy<Mutex<Option<ShareDebugMap>>> = Lazy::new(|| Mutex::new(None));
static BRANCHES: Lazy<Mutex<Option<Branches>>> = Lazy::new(|| Mutex::new(None));
static STAGED_FILES: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone)]
pub struct Branches {
    pub available: Vec<String>,
    pub current: String,
}

#[derive(Debug, Clone, Copy, Display)]
pub enum Loading {
    #[strum(serialize = "Nuking storage")]
    Nuking,
    #[strum(serialize = "Loading git repository")]
    GitRepository,
    #[strum(serialize = "Loading git branches")]
    GitBranches,
    #[strum(serialize = "Loading current git branch")]
    GitBranch,
    #[strum(serialize = "Loading animations")]
    Animations,
    #[strum(serialize = "Loading curves")]
    Curves,
    #[strum(serialize = "Loading output devices")]
    OutputDevices,
    #[strum(serialize = "Loading palettes")]
    Palettes,
    #[strum(serialize = "Loading projects")]
    Projects,
    #[strum(serialize = "Loading scenes")]
    Scenes,
}

impl Loading {
    pub fn set(self) {
        *LOADING.lock() = Some(self);
    }

    pub fn unset() {
        LOADING.lock().take();
    }
}

pub fn working() -> bool {
    WORKING.load(Relaxed)
}

pub fn staged_files() -> usize {
    STAGED_FILES.load(Relaxed)
}

pub fn loading() -> Option<Loading> {
    *LOADING.lock()
}

pub fn error() -> Option<String> {
    ERROR.lock().clone()
}

pub fn branches() -> Option<Branches> {
    BRANCHES.lock().clone()
}

pub fn start_thread() {
    let actions = action::init();

    std::thread::spawn(move || {
        let mut retry_wait = std::time::Duration::from_secs(0);
        loop {
            sleep(retry_wait);
            retry_wait = std::time::Duration::from_secs(2);

            ERROR.lock().take();

            // Clear the queue
            while actions.try_recv().is_ok() {}

            Loading::GitRepository.set();
            let mut git = match git::Git::open(PersistantState::git_url()) {
                Ok(git) => git,
                Err(err) => {
                    let err: String = format!("Could not open git: {err}");
                    log::error!("{err}");
                    ERROR.lock().replace(err);
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
                        ERROR.lock().take();
                        Loading::Nuking.set();
                        remove_dir_all(&*STORAGE_DIR).ok();
                        Action::Restart.enqueue();
                    }
                    Action::Restart => {
                        break;
                    }
                    Action::CountStagedFiles => match git.count_staged_files() {
                        Err(err) => {
                            ERROR
                                .lock()
                                .replace(format!("Error counting staged files: {err}"));
                            sleep(Duration::from_secs(1));
                            Action::CountStagedFiles.enqueue();
                        }
                        Ok(count) => {
                            STAGED_FILES.store(count, Relaxed);
                            ERROR.lock().take();
                        }
                    },
                    Action::Pull => {
                        if let Err(err) = git.pull() {
                            ERROR.lock().replace(format!("Error pulling: {err}"));
                            sleep(Duration::from_secs(1));
                            Action::Pull.enqueue();
                        } else {
                            ERROR.lock().take();
                        }
                    }
                    Action::CommitAndPush { message } => {
                        if let Err(err) = git.commit_and_push(&message) {
                            ERROR
                                .lock()
                                .replace(format!("Error committing and pushing: {err}"));
                            sleep(Duration::from_secs(1));
                            Action::CommitAndPush { message }.enqueue();
                        } else {
                            ERROR.lock().take();
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
                                ERROR.lock().replace(err);
                                continue;
                            }
                        };

                        Loading::GitBranch.set();
                        let current_branch = match git.current_branch() {
                            Ok(current_branch) => current_branch,
                            Err(err) => {
                                let err: String = format!("Could not get current_branch: {err}");
                                log::error!("{err}");
                                ERROR.lock().replace(err);
                                continue;
                            }
                        };

                        *BRANCHES.lock() = Some(Branches {
                            available: branches,
                            current: current_branch,
                        });

                        ERROR.lock().take();
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

                        COLLECTIONS.lock().replace(collections);
                        Loading::unset();
                    }
                    Action::SwitchBranch(branch) => match git.switch_branch(&branch) {
                        Ok(_) => {
                            Action::Restart.enqueue();
                            Action::LoadBranches.enqueue();
                            Action::CountStagedFiles.enqueue();
                            Action::LoadAssets.enqueue();
                        }
                        Err(err) => {
                            ERROR
                                .lock()
                                .replace(format!("Error switching branch: {err}"));
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
