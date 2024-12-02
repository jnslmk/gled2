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
    path::{Path, PathBuf},
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
static STATE: Lazy<Mutex<State>> = Lazy::new(|| Mutex::new(State::Loading(Loading::GitRepository)));
static BRANCHES: Lazy<Mutex<Option<Branches>>> = Lazy::new(|| Mutex::new(None));

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
    Opened {
        synced: bool,
        #[allow(unused)]
        folder: PathBuf,
        collections: ShareDebugMap,
    },
}

impl State {
    pub fn set(self) {
        match &self {
            Self::Opened { .. } => println!("Storage is fully loaded."),
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

pub fn staged_changes() -> usize {
    //TODO: Check if there's sth to commit
    if opened() {
        1
    } else {
        0
    }
}

pub fn opened() -> bool {
    matches!(*STATE.lock(), State::Opened { .. })
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
            Loading::GitRepository.set();

            std::thread::sleep(retry_wait);
            retry_wait = std::time::Duration::from_secs(2);

            let mut git = match git::Git::open(PersistantState::git_url()) {
                Ok(git) => git,
                Err(err) => {
                    let err: String = format!("Could not open git: {err}");
                    log::error!("{err}");
                    *STATE.lock() = State::Error(err);
                    continue;
                }
            };

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

            Loading::Animations.set();

            let root = git.folder().to_owned();
            let mut collections = ShareDebugMap::custom();
            collections.insert::<Collection<Animation>>(Collection::<Animation>::load(&root));

            Loading::Curves.set();
            collections.insert::<Collection<Curve>>(Collection::<Curve>::load(&root));

            Loading::OutputDevices.set();
            collections.insert::<Collection<OutputDevice>>(Collection::<OutputDevice>::load(&root));

            Loading::Palettes.set();
            collections.insert::<Collection<Palette>>(Collection::<Palette>::load(&root));

            Loading::Projects.set();
            collections.insert::<Collection<Project>>(Collection::<Project>::load(&root));

            Loading::Scenes.set();
            collections.insert::<Collection<Scene>>(Collection::<Scene>::load(&root));

            State::Opened {
                synced: git.synced(),
                folder: root.clone(),
                collections,
            }
            .set();

            while let Ok(action) = actions.recv() {
                match action {
                    Action::CommitAndPush { message } => {
                        if let Err(err) = git.commit_and_push(&message) {
                            State::Error(format!("Error committing and pushing: {err}")).set();
                        }
                    }
                    Action::Restart => {
                        BRANCHES.lock().take();
                        println!("Restarting Storage");
                        break;
                    }
                    Action::SwitchBranch(branch) => match git.switch_branch(&branch) {
                        Ok(_) => {
                            Action::Restart.enqueue();
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
                        if let Err(err) = git.write_asset(&asset_path(&root, uuid, dir_name), json)
                        {
                            log::error!("Could not write asset: {err:?}");
                            continue;
                        }

                        let state: &mut State = &mut STATE.lock();
                        if let State::Opened { synced, .. } = state {
                            *synced = git.synced();
                        }
                    }
                    Action::DeleteAsset { uuid, dir_name } => {
                        if let Err(err) = git.delete_asset(&asset_path(&root, uuid, dir_name)) {
                            log::error!("Could not delete asset: {err:?}");
                            continue;
                        }

                        let state: &mut State = &mut STATE.lock();
                        if let State::Opened { synced, .. } = state {
                            *synced = git.synced();
                        }
                    }
                }
            }
        }
    });
}

pub fn asset_path(root: &Path, id: Uuid, dir_name: &str) -> PathBuf {
    root.join(dir_name).join(format!("{}.json", id))
}
