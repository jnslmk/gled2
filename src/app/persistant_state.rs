use crate::{
    storage::{
        asset::{palette::Palette, project::Project},
        asset_id::AssetId,
        git::GitCredentials,
    },
    ui::action::UiAction,
};
use egui::mutex::Mutex;
use log::info;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

static PERSISTANT_STATE: Lazy<Mutex<PersistantState>> =
    Lazy::new(|| Mutex::new(PersistantState::load()));

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct PersistantState {
    pub effects_size: f32,
    pub effects_always_render: bool,
    pub fps_limit: f32,
    pub always_render_fps: f32,
    pub preview_palette: Option<AssetId<Palette>>,
    pub last_project_id: Option<AssetId<Project>>,
    pub git_url: String,
    pub git_credentials: GitCredentials,
    pub prefer_discrete_gpu: bool,
}

impl Default for PersistantState {
    fn default() -> Self {
        Self {
            effects_size: 100.0,
            effects_always_render: false,
            fps_limit: 120.0,
            always_render_fps: 0.0,
            preview_palette: Default::default(),
            last_project_id: Default::default(),
            git_url: "https://gitlab.com/photonenkollektiv/gled2_assets.git".to_string(),
            git_credentials: Default::default(),
            prefer_discrete_gpu: true,
        }
    }
}

impl PersistantState {
    fn load() -> Self {
        Self::path()
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|contents| {
                serde_json::from_slice(&contents)
                    .map_err(|err| log::warn!("Could not parse {:?}: {err:?}", Self::path()))
                    .ok()
            })
            .unwrap_or_default()
    }

    pub fn fps_limit() -> f32 {
        PERSISTANT_STATE.lock().fps_limit
    }

    pub fn always_render_fps() -> f32 {
        PERSISTANT_STATE.lock().always_render_fps
    }

    pub fn effects_always_render() -> bool {
        PERSISTANT_STATE.lock().effects_always_render
    }

    pub fn effects_size() -> f32 {
        PERSISTANT_STATE.lock().effects_size
    }

    pub fn git_url() -> String {
        PERSISTANT_STATE.lock().git_url.clone()
    }

    pub fn prefer_discrete_gpu() -> bool {
        PERSISTANT_STATE.lock().prefer_discrete_gpu
    }

    pub fn get() -> Self {
        PERSISTANT_STATE.lock().clone()
    }

    pub fn git_credentials() -> GitCredentials {
        PERSISTANT_STATE.lock().git_credentials.clone()
    }

    pub fn save(self) {
        *PERSISTANT_STATE.lock() = self.clone();

        std::thread::spawn(move || {
            let Some(path) = Self::path() else {
                UiAction::Error("Could not determine persistant state path".to_string()).enqueue();
                return;
            };
            let Ok(contents) = serde_json::to_string_pretty(&self) else {
                UiAction::Error("Could not serialize persistant state".to_string()).enqueue();
                return;
            };

            if let Err(err) = std::fs::write(path, contents) {
                UiAction::Error(format!("Could not persist state: {err:?}")).enqueue();
            }

            info!("Saved persistant state");
        });
    }

    fn path() -> Option<PathBuf> {
        home::home_dir().map(|path| path.join(".gled"))
    }
}
