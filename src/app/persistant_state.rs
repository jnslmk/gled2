use super::effects::Effects;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub struct PersistantState {
    pub show_preview_svg: bool,
    pub main_dimmer: f32,
    pub fullscreen: bool,
    pub effects: Effects,
    pub fps_limit: f32,
    pub preview_height: f32,
    #[serde(skip)]
    pub dirty: bool,
}

impl Default for PersistantState {
    fn default() -> Self {
        Self {
            show_preview_svg: true,
            main_dimmer: 1.0,
            fullscreen: Default::default(),
            effects: Default::default(),
            fps_limit: 120.0,
            preview_height: 300.0,
            dirty: true,
        }
    }
}

impl PersistantState {
    pub fn load() -> Self {
        Self::path()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|contents| serde_json::from_str(&contents).ok())
            .unwrap_or_default()
    }

    pub fn store(&mut self) {
        if !self.dirty {
            return;
        }

        self.dirty = false;
        let persistant_state = self.clone();
        std::thread::spawn(move || {
            let Some(path) = Self::path() else {
                error!("Could not determine persistant state path");
                return;
            };
            let Ok(contents) = serde_json::to_string_pretty(&persistant_state) else {
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
