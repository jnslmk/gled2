mod action;
mod asset;
mod asset_id;
mod collection;
mod git;
mod palette;
mod project;
mod scene;

use collection::Collection;
use egui::mutex::Mutex;
use once_cell::sync::Lazy;
use std::{fmt::Debug, path::PathBuf, sync::Arc};

pub use self::{
    action::Action,
    asset::{Asset, AssetTrait},
    asset_id::AssetId,
    palette::Palette,
    project::Project,
    scene::Scene,
};

static STATE: Lazy<Mutex<State>> = Lazy::new(|| Mutex::new(State::Loading(0.0)));

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum State {
    Loading(f32),
    Error(String),
    Opened {
        synced: bool,
        branches: Vec<String>,
        current_branch: String,
        folder: PathBuf,
        projects: Collection<Project>,
        palettes: Collection<Palette>,
        scenes: Collection<Scene>,
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
                match git::Git::open("git@git.freshx.de:rene/gled2_assets.git".to_string()) {
                    Ok(git) => git,
                    Err(err) => {
                        *STATE.lock() = State::Error(format!("Could not open git: {err}"));
                        continue;
                    }
                };

            *STATE.lock() = State::Loading(0.2);

            let branches = match git.branches() {
                Ok(branches) => branches,
                Err(err) => {
                    *STATE.lock() = State::Error(format!("Could not get branches: {err}"));
                    continue;
                }
            };

            *STATE.lock() = State::Loading(0.3);

            let current_branch = match git.current_branch() {
                Ok(current_branch) => current_branch,
                Err(err) => {
                    *STATE.lock() = State::Error(format!("Could not get current_branch: {err}"));
                    continue;
                }
            };

            *STATE.lock() = State::Loading(0.4);

            let root = git.folder().to_owned();

            let palettes = Collection::<Palette>::load(root.join("palettes"));
            *STATE.lock() = State::Loading(0.6);
            let projects = Collection::<Project>::load(root.join("projects"));
            *STATE.lock() = State::Loading(0.8);
            let scenes = Collection::<Scene>::load(root.join("scenes"));
            *STATE.lock() = State::Loading(1.0);

            *STATE.lock() = State::Opened {
                synced: git.synced(),
                branches,
                current_branch,
                folder: root.clone(),
                projects,
                palettes,
                scenes,
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
                    Action::SavePalette { palette } => {
                        if git
                            .write_asset(&palette.id.disk_path(&root), &root, palette)
                            .is_err()
                        {
                            log::error!("Could not save color palette");
                            continue;
                        }

                        let state: &mut State = &mut STATE.lock();
                        if let State::Opened { synced, .. } = state {
                            *synced = git.synced();
                        }
                    }
                    Action::SaveProject { project } => {
                        if git
                            .write_asset(&project.id.disk_path(&root), &root, project)
                            .is_err()
                        {
                            log::error!("Could not save project");
                            continue;
                        }

                        let state: &mut State = &mut STATE.lock();
                        if let State::Opened { synced, .. } = state {
                            *synced = git.synced();
                        }
                    }
                    Action::SaveScene { scene } => {
                        if let Err(err) = git.write_asset(&scene.id.disk_path(&root), &root, scene)
                        {
                            log::error!("Could not save scene: {err}");
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

fn find_palette(id: &AssetId<Palette>) -> Arc<Asset<Palette>> {
    let state: &State = &STATE.lock();
    if let State::Opened { palettes, .. } = state {
        if let Some(asset) = palettes.get(id) {
            return asset.clone();
        }
    }

    Arc::new(Asset {
        id: id.to_owned(),
        name: Default::default(),
        data: Palette::default(),
    })
}

fn set_palette_in_cache(palette: Asset<Palette>) {
    log::info!("Setting palette in cache: {:?}", palette.id);
    let state: &mut State = &mut STATE.lock();
    if let State::Opened { palettes, .. } = state {
        palettes.set_asset(palette);
    }
}

fn find_project(id: &AssetId<Project>) -> Arc<Asset<Project>> {
    let state: &State = &STATE.lock();
    if let State::Opened { projects, .. } = state {
        if let Some(asset) = projects.get(id) {
            return asset.clone();
        }
    }

    Arc::new(Asset {
        id: id.to_owned(),
        name: Default::default(),
        data: Project::default(),
    })
}

fn set_project_in_cache(project: Asset<Project>) {
    log::info!("Setting project in cache: {:?}", project.id);
    let state: &mut State = &mut STATE.lock();
    if let State::Opened { projects, .. } = state {
        projects.set_asset(project);
    }
}

fn find_scene(id: &AssetId<Scene>) -> Arc<Asset<Scene>> {
    let state: &State = &STATE.lock();
    if let State::Opened { scenes, .. } = state {
        if let Some(asset) = scenes.get(id) {
            return asset.clone();
        }
    }

    Arc::new(Asset {
        id: id.to_owned(),
        name: Default::default(),
        data: Scene::default(),
    })
}

fn set_scene_in_cache(scene: Asset<Scene>) {
    log::info!("Setting scene in cache: {:?}", scene.id);
    let state: &mut State = &mut STATE.lock();
    if let State::Opened { scenes, .. } = state {
        scenes.set_asset(scene);
    }
}
