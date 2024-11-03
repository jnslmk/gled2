use crate::input::{GamepadEvent, InputEvent};
use egui::mutex::Mutex;
use log::{error, info};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::PathBuf};

static PERSISTANT_STATE: Lazy<Mutex<PersistantState>> =
    Lazy::new(|| Mutex::new(PersistantState::load()));

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct PersistantState {
    pub show_preview_svg: bool,
    pub main_dimmer: f32,
    pub fullscreen: bool,
    pub effects_size: f32,
    pub effects_show_svg: bool,
    pub effects_always_render: bool,
    pub fps_limit: f32,
    pub preview_height: f32,
    pub tap_input_events: BTreeSet<InputEvent>,
    pub freeze_input_events: BTreeSet<InputEvent>,
    pub blackout_input_events: BTreeSet<InputEvent>,
}

impl Default for PersistantState {
    fn default() -> Self {
        Self {
            show_preview_svg: true,
            main_dimmer: 1.0,
            fullscreen: Default::default(),
            effects_size: 100.0,
            effects_show_svg: true,
            effects_always_render: false,
            fps_limit: 120.0,
            preview_height: 300.0,
            tap_input_events: std::iter::once(InputEvent::Key(egui::Key::T))
                .chain(std::iter::once(InputEvent::Gamepad(GamepadEvent::Mode(0))))
                .collect(),
            freeze_input_events: std::iter::once(InputEvent::Key(egui::Key::F))
                .chain(std::iter::once(InputEvent::Gamepad(GamepadEvent::Select(
                    0,
                ))))
                .collect(),
            blackout_input_events: std::iter::once(InputEvent::Key(egui::Key::B))
                .chain(std::iter::once(InputEvent::Gamepad(GamepadEvent::Start(0))))
                .collect(),
        }
    }
}

impl PersistantState {
    fn load() -> Self {
        Self::path()
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|mut contents| simd_json::from_slice(&mut contents).ok())
            .unwrap_or_default()
    }

    pub fn fps_limit() -> f32 {
        PERSISTANT_STATE.lock().fps_limit
    }

    pub fn main_dimmer() -> f32 {
        PERSISTANT_STATE.lock().main_dimmer
    }

    pub fn effects_always_render() -> bool {
        PERSISTANT_STATE.lock().effects_always_render
    }

    pub fn effects_size() -> f32 {
        PERSISTANT_STATE.lock().effects_size
    }

    pub fn effects_show_svg() -> bool {
        PERSISTANT_STATE.lock().effects_show_svg
    }

    pub fn show_preview_svg() -> bool {
        PERSISTANT_STATE.lock().show_preview_svg
    }

    pub fn preview_height() -> f32 {
        PERSISTANT_STATE.lock().preview_height
    }

    pub fn tap_input_is_new() -> bool {
        PERSISTANT_STATE
            .lock()
            .tap_input_events
            .iter()
            .any(|event| event.is_new())
    }

    pub fn freeze_input_is_new() -> bool {
        PERSISTANT_STATE
            .lock()
            .freeze_input_events
            .iter()
            .any(|event| event.is_new())
    }

    pub fn blackout_input_is_new() -> bool {
        PERSISTANT_STATE
            .lock()
            .blackout_input_events
            .iter()
            .any(|event| event.is_new())
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
            let Ok(contents) = simd_json::to_string_pretty(&self) else {
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
