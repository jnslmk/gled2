use crate::storage::{AssetId, Palette, Project};
use egui::mutex::Mutex;
use log::{error, info};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

static PERSISTANT_STATE: Lazy<Mutex<PersistantState>> =
    Lazy::new(|| Mutex::new(PersistantState::load()));

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct PersistantState {
    pub show_preview_svg: bool,
    pub fullscreen: bool,
    pub effects_size: f32,
    pub effects_always_render: bool,
    pub fps_limit: f32,
    pub preview_palette: Option<AssetId<Palette>>,
    pub last_project_id: Option<AssetId<Project>>,
}

impl Default for PersistantState {
    fn default() -> Self {
        Self {
            show_preview_svg: true,
            fullscreen: Default::default(),
            effects_size: 100.0,
            effects_always_render: false,
            fps_limit: 120.0,
            preview_palette: Default::default(),
            last_project_id: Default::default(),
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

    pub fn effects_always_render() -> bool {
        PERSISTANT_STATE.lock().effects_always_render
    }

    pub fn effects_size() -> f32 {
        PERSISTANT_STATE.lock().effects_size
    }

    pub fn show_preview_svg() -> bool {
        PERSISTANT_STATE.lock().show_preview_svg
    }

    pub fn get() -> Self {
        PERSISTANT_STATE.lock().clone()
    }

    pub fn save(self) {
        *PERSISTANT_STATE.lock() = self.clone();

        std::thread::spawn(move || {
            let Some(path) = Self::path() else {
                error!("Could not determine persistant state path");
                return;
            };
            let Ok(contents) = serde_json::to_string_pretty(&self) else {
                error!("Could not serialize persistant state");
                return;
            };

            if let Err(err) = std::fs::write(path, contents) {
                error!("Could not persist state: {err:?}");
            }

            info!("Saved persistant state");
        });
    }

    fn path() -> Option<PathBuf> {
        home::home_dir().map(|path| path.join(".gled"))
    }
}
