//! Renders to a texture

mod blob;
mod colors;
mod config;
mod gradient;
mod renderer;
mod spiral;
mod state;
mod stripes;

use gled_proc_macros::Animation;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use strum::{Display, EnumIter};

pub use blob::Blob;
pub use colors::{Color, ColorPalette};
pub use config::{CommonConfig, Config, Direction};
pub use gradient::{Gradient, GradientType};
pub use renderer::AnimationRenderer;
pub use spiral::Spiral;
pub use state::State;
pub use stripes::Stripes;

pub trait AnimationConfig: Into<Animation> + Default + Debug + Clone {
    fn ui(&mut self, ui: &mut egui::Ui, _texture_id: egui::TextureId);
    fn shader_code(&self) -> std::borrow::Cow<str>;
    fn config(&self) -> Config;
}

#[derive(Serialize, Deserialize, Debug, Clone, Animation, EnumIter, Display, PartialEq, Eq)]
pub enum Animation {
    Blob(blob::Blob),
    Gradient(gradient::Gradient),
    Stripes(stripes::Stripes),
    Spiral(spiral::Spiral),
}

impl Default for Animation {
    fn default() -> Self {
        Self::Gradient(Gradient::default())
    }
}
