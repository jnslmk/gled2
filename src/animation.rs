//! Renders to a texture

mod colors;
mod config;
mod gradient;
mod renderer;
mod state;
mod stripes;

use gled_proc_macros::Animation;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub use colors::{Color, ColorPalette};
pub use config::{CommonConfig, Config, Direction};
pub use gradient::{Gradient, GradientType};
pub use renderer::AnimationRenderer;
pub use state::State;
pub use stripes::Stripes;

pub trait AnimationConfig: Into<Animation> + Default + Debug + Clone {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.label("TODO");
    }

    fn shader_code(&self) -> std::borrow::Cow<str>;
    fn config(&self) -> Config;
}

#[derive(Serialize, Deserialize, Debug, Clone, Animation)]
pub enum Animation {
    Gradient(gradient::Gradient),
    Stripes(stripes::Stripes),
}

impl Default for Animation {
    fn default() -> Self {
        Self::Gradient(Gradient::default())
    }
}
