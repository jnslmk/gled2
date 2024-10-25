mod action;
mod git;
mod palette;
mod path;
mod project;
mod scene;
mod tree;

use self::action::Action;
use std::{
    fmt::Debug,
    path::PathBuf,
    sync::{Arc, RwLock},
};
use tree::Folders;

pub use self::{
    palette::Palette,
    path::AssetPath,
    project::Project,
    scene::Scene,
    tree::{Asset, AssetTrait, ChangingAsset},
};

static STATE: RwLock<State> = RwLock::new(State::Loading(0.0));

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
        projects: Folders<Project>,
        palettes: Folders<Palette>,
        scenes: Folders<Scene>,
    },
}

pub fn loading_state() -> Option<f32> {
    if let State::Loading(loading) = *STATE.read().unwrap() {
        Some(loading)
    } else {
        None
    }
}

pub fn error_state() -> Option<String> {
    if let State::Error(err) = &*STATE.read().unwrap() {
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
            *STATE.write().unwrap() = State::Loading(0.0);

            std::thread::sleep(retry_wait);
            retry_wait = std::time::Duration::from_secs(2);

            let mut git =
                match git::Git::open("git@git.freshx.de:rene/gled2_assets.git".to_string()) {
                    Ok(git) => git,
                    Err(err) => {
                        *STATE.write().unwrap() =
                            State::Error(format!("Could not open git: {err}"));
                        continue;
                    }
                };

            *STATE.write().unwrap() = State::Loading(0.2);

            let branches = match git.branches() {
                Ok(branches) => branches,
                Err(err) => {
                    *STATE.write().unwrap() =
                        State::Error(format!("Could not get branches: {err}"));
                    continue;
                }
            };

            *STATE.write().unwrap() = State::Loading(0.3);

            let current_branch = match git.current_branch() {
                Ok(current_branch) => current_branch,
                Err(err) => {
                    *STATE.write().unwrap() =
                        State::Error(format!("Could not get current_branch: {err}"));
                    continue;
                }
            };

            *STATE.write().unwrap() = State::Loading(0.4);

            let root = git.folder().to_owned();

            let palettes = Folders::<Palette>::load(root.join("palettes"));
            *STATE.write().unwrap() = State::Loading(0.6);
            let projects = Folders::<Project>::load(root.join("projects"));
            *STATE.write().unwrap() = State::Loading(0.8);
            let scenes = Folders::<Scene>::load(root.join("scenes"));
            *STATE.write().unwrap() = State::Loading(1.0);

            *STATE.write().unwrap() = State::Opened {
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
                            *STATE.write().unwrap() =
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
                            *STATE.write().unwrap() =
                                State::Error(format!("Error switching branch: {err}"));
                            break;
                        }
                    },
                    Action::SavePalette { palette } => {
                        if find_palette(&palette.path) != palette {
                            log::info!("Skipping save as a new version is already in cache");
                            continue;
                        }

                        if git
                            .write_file(&palette.path.disk_path(&root), &root, &palette)
                            .is_err()
                        {
                            log::error!("Could not save color palette {:?}", palette.path);
                            continue;
                        }

                        let state: &mut State = &mut STATE.write().unwrap();
                        if let State::Opened { synced, .. } = state {
                            *synced = git.synced();
                        }
                    }
                    Action::SaveProject { project } => {
                        if find_project(&project.path) != project {
                            log::info!("Skipping save as a new version is already in cache");
                            continue;
                        }

                        if git
                            .write_file(&project.path.disk_path(&root), &root, &project)
                            .is_err()
                        {
                            log::error!("Could not save project {:?}", project.path);
                            continue;
                        }

                        let state: &mut State = &mut STATE.write().unwrap();
                        if let State::Opened { synced, .. } = state {
                            *synced = git.synced();
                        }
                    }
                    Action::SaveScene { scene } => {
                        if find_scene(&scene.path) != scene {
                            log::info!("Skipping save as a new version is already in cache");
                            continue;
                        }

                        if git
                            .write_file(&scene.path.disk_path(&root), &root, &scene)
                            .is_err()
                        {
                            log::error!("Could not save scene {:?}", scene.path);
                            continue;
                        }

                        let state: &mut State = &mut STATE.write().unwrap();
                        if let State::Opened { synced, .. } = state {
                            *synced = git.synced();
                        }
                    }
                }
            }
        }
    });
}

fn find_palette(path: &AssetPath) -> Asset<Palette> {
    let state: &State = &STATE.read().unwrap();
    if let State::Opened { palettes, .. } = state {
        if let Some(asset) = palettes.get(path) {
            return asset.clone();
        }
    }

    Asset {
        path: path.to_owned(),
        data: Arc::new(Palette::default()),
    }
}

fn set_palette_in_cache(palette: Asset<Palette>) {
    log::info!("Setting palette in cache: {:?}", palette.path);
    let state: &mut State = &mut STATE.write().unwrap();
    if let State::Opened { palettes, .. } = state {
        palettes.set_asset(palette);
    }
}

fn find_project(path: &AssetPath) -> Asset<Project> {
    let state: &State = &STATE.read().unwrap();
    if let State::Opened { projects, .. } = state {
        if let Some(asset) = projects.get(path) {
            return asset.clone();
        }
    }

    Asset {
        path: path.to_owned(),
        data: Arc::new(Project::default()),
    }
}

fn set_project_in_cache(project: Asset<Project>) {
    log::info!("Setting project in cache: {:?}", project.path);
    let state: &mut State = &mut STATE.write().unwrap();
    if let State::Opened { projects, .. } = state {
        projects.set_asset(project);
    }
}

fn find_scene(path: &AssetPath) -> Asset<Scene> {
    let state: &State = &STATE.read().unwrap();
    if let State::Opened { scenes, .. } = state {
        if let Some(asset) = scenes.get(path) {
            return asset.clone();
        }
    }

    Asset {
        path: path.to_owned(),
        data: Arc::new(Scene::default()),
    }
}

fn set_scene_in_cache(scene: Asset<Scene>) {
    log::info!("Setting scene in cache: {:?}", scene.path);
    let state: &mut State = &mut STATE.write().unwrap();
    if let State::Opened { scenes, .. } = state {
        scenes.set_asset(scene);
    }
}
