pub mod action;
pub mod asset;
pub mod asset_id;
pub mod collection;
pub mod git;

use crate::app::persistant_state::PersistantState;

use self::{action::StorageAction, asset::*, asset_id::AssetId};
use animation::Animation;
use collection::Collection;
use curve::Curve;
use directories::BaseDirs;
use egui::mutex::Mutex;
use once_cell::sync::Lazy;
use output_device::OutputDevice;
use palette::Palette;
use project::Project;
use scene::Scene;
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

            StorageAction::LoadBranches.enqueue();
            StorageAction::CountStagedFiles.enqueue();
            StorageAction::LoadAssets.enqueue();

            while let Ok(action) = actions.recv() {
                WORKING.store(true, Relaxed);

                match action {
                    StorageAction::Nuke => {
                        ERROR.lock().take();
                        Loading::Nuking.set();
                        remove_dir_all(&*STORAGE_DIR).ok();
                        StorageAction::Restart.enqueue();
                    }
                    StorageAction::Restart => {
                        break;
                    }
                    StorageAction::CountStagedFiles => match git.count_staged_files() {
                        Err(err) => {
                            ERROR
                                .lock()
                                .replace(format!("Error counting staged files: {err}"));
                            sleep(Duration::from_secs(1));
                            StorageAction::CountStagedFiles.enqueue();
                        }
                        Ok(count) => {
                            STAGED_FILES.store(count, Relaxed);
                            ERROR.lock().take();
                        }
                    },
                    StorageAction::Pull => {
                        if let Err(err) = git.pull() {
                            ERROR.lock().replace(format!("Error pulling: {err}"));
                            sleep(Duration::from_secs(1));
                            StorageAction::Pull.enqueue();
                        } else {
                            ERROR.lock().take();
                        }
                    }
                    StorageAction::Push => {
                        if let Err(err) = git.push() {
                            ERROR.lock().replace(format!("Error pushing: {err}"));
                            sleep(Duration::from_secs(1));
                            StorageAction::Push.enqueue();
                        } else {
                            ERROR.lock().take();
                        }
                    }
                    StorageAction::Commit { message } => {
                        if let Err(err) = git.commit(&message) {
                            ERROR
                                .lock()
                                .replace(format!("Error committing and pushing: {err}"));
                            sleep(Duration::from_secs(1));
                            StorageAction::Commit { message }.enqueue();
                        } else {
                            ERROR.lock().take();
                        }
                        StorageAction::CountStagedFiles.enqueue();
                    }
                    StorageAction::LoadBranches => {
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
                    StorageAction::LoadAssets => {
                        if let Err(err) = mkdirp::mkdirp(STORAGE_DIR.join("svg")) {
                            ERROR
                                .lock()
                                .replace(format!("Could not create svg templates folder: {err}"));
                            continue;
                        }

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
                    StorageAction::SwitchBranch(branch) => match git.switch_branch(&branch) {
                        Ok(_) => {
                            StorageAction::Restart.enqueue();
                            StorageAction::LoadBranches.enqueue();
                            StorageAction::CountStagedFiles.enqueue();
                            StorageAction::LoadAssets.enqueue();
                        }
                        Err(err) => {
                            ERROR
                                .lock()
                                .replace(format!("Error switching branch: {err}"));
                        }
                    },
                    StorageAction::SaveAsset {
                        dir_name,
                        uuid,
                        json,
                    } => {
                        if let Err(err) = git.write_asset(&asset_path(uuid, dir_name), json) {
                            log::error!("Could not write asset: {err:?}");
                        }

                        StorageAction::CountStagedFiles.enqueue();
                    }
                    StorageAction::DeleteAsset { uuid, dir_name } => {
                        if let Err(err) = git.delete_asset(&asset_path(uuid, dir_name)) {
                            log::error!("Could not delete asset: {err:?}");
                        }

                        StorageAction::CountStagedFiles.enqueue();
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
