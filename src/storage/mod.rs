mod action;
mod asset;
mod asset_id;
mod collection;
mod git;

use collection::Collection;
use egui::mutex::Mutex;
use once_cell::sync::Lazy;
use std::{
    fmt::Debug,
    path::{Path, PathBuf},
};
use typemap::ShareDebugMap;
use uuid::Uuid;

pub use self::{action::Action, asset::*, asset_id::AssetId};

static STATE: Lazy<Mutex<State>> = Lazy::new(|| Mutex::new(State::Loading(0.0)));

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum State {
    Loading(f32),
    Error(String),
    Opened {
        synced: bool,
        #[allow(unused)]
        branches: Vec<String>,
        #[allow(unused)]
        current_branch: String,
        #[allow(unused)]
        folder: PathBuf,
        collections: ShareDebugMap,
    },
}

pub fn loading_state() -> Option<f32> {
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

pub fn start_thread() {
    let actions = action::init();

    std::thread::spawn(move || {
        let mut retry_wait = std::time::Duration::from_secs(0);
        loop {
            *STATE.lock() = State::Loading(0.0);

            std::thread::sleep(retry_wait);
            retry_wait = std::time::Duration::from_secs(2);

            let mut git =
                match git::Git::open("git@gitlab.com:pentagonum/gled2_assets.git".to_string()) {
                    Ok(git) => git,
                    Err(err) => {
                        let err: String = format!("Could not open git: {err}");
                        log::error!("{err}");
                        *STATE.lock() = State::Error(err);
                        continue;
                    }
                };

                log::debug!("Git opened");
            *STATE.lock() = State::Loading(0.2);

            let branches = match git.branches() {
                Ok(branches) => branches,
                Err(err) => {
                    let err: String = format!("Could not get branches: {err}");
                    log::error!("{err}");
                    *STATE.lock() = State::Error(err);
                    continue;
                }
            };

            log::debug!("Got branches");
            *STATE.lock() = State::Loading(0.3);

            let current_branch = match git.current_branch() {
                Ok(current_branch) => current_branch,
                Err(err) => {
                    let err: String = format!("Could not get current_branch: {err}");
                    log::error!("{err}");
                    *STATE.lock() = State::Error(err);
                    continue;
                }
            };

            log::debug!("Got current branch");
            *STATE.lock() = State::Loading(0.4);

            let root = git.folder().to_owned();
            let mut collections = ShareDebugMap::custom();

            collections.insert::<Collection<Animation>>(Collection::<Animation>::load(&root));
            *STATE.lock() = State::Loading(0.5);
            collections.insert::<Collection<Curve>>(Collection::<Curve>::load(&root));
            *STATE.lock() = State::Loading(0.6);
            collections.insert::<Collection<OutputDevice>>(Collection::<OutputDevice>::load(&root));
            *STATE.lock() = State::Loading(0.7);
            collections.insert::<Collection<Palette>>(Collection::<Palette>::load(&root));
            *STATE.lock() = State::Loading(0.8);
            collections.insert::<Collection<Project>>(Collection::<Project>::load(&root));
            *STATE.lock() = State::Loading(0.9);
            collections.insert::<Collection<Scene>>(Collection::<Scene>::load(&root));
            *STATE.lock() = State::Loading(1.0);

            *STATE.lock() = State::Opened {
                synced: git.synced(),
                branches,
                current_branch,
                folder: root.clone(),
                collections,
            };

            //TODO: Loader before main window which turns into main window once we are here

            while let Ok(action) = actions.recv() {
                match action {
                    Action::CommitAndPush { message } => {
                        if let Err(err) = git.commit_and_push(&message) {
                            *STATE.lock() =
                                State::Error(format!("Error committing and pushing: {err}"));
                            break;
                        }
                    }
                    Action::Update => break,
                    Action::SwitchBranch(branch) => match git.switch_branch(&branch) {
                        Ok(_) => {
                            // needs to reload all assets
                            break;
                        }
                        Err(err) => {
                            *STATE.lock() = State::Error(format!("Error switching branch: {err}"));
                            break;
                        }
                    },
                    Action::SaveAsset {
                        dir_name,
                        uuid,
                        json,
                    } => {
                        if let Err(err) = git.write_asset(&asset_path(&root, uuid, dir_name), json)
                        {
                            log::error!("Could not save color palette: {err:?}");
                            continue;
                        }

                        let state: &mut State = &mut STATE.lock();
                        if let State::Opened { synced, .. } = state {
                            *synced = git.synced();
                        }
                    }
                    Action::DeleteAsset { uuid, dir_name } => {
                        if let Err(err) = git.delete_asset(&asset_path(&root, uuid, dir_name)) {
                            log::error!("Could not delete color palette: {err:?}");
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
