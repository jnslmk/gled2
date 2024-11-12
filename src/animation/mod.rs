//! Renders to a texture

mod blob;
mod config;
mod gradient;
mod opposing_lines;
mod random_circles;
mod renderer;
mod rotating_square;
mod set_center;
mod spiral;
mod stripes;

use gled_proc_macros::Animation;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use strum::{Display, EnumIter};

pub use blob::Blob;
pub use config::Config;
pub use gradient::Gradient;
pub use opposing_lines::OpposingLines;
pub use random_circles::RandomCircles;
pub use renderer::AnimationRenderer;
pub use rotating_square::RotatingSquare;
pub use spiral::Spiral;
pub use stripes::Stripes;

pub trait AnimationConfig: Into<Animation> + Default + Debug + Clone {
    fn ui(&mut self, ui: &mut egui::Ui, rendered: egui::TextureId);
    fn shader_code(&self) -> std::borrow::Cow<str>;
    fn config(&self) -> Config;
    fn uses_multiple_colors(&self) -> bool;
}

#[derive(Serialize, Deserialize, Debug, Clone, Animation, EnumIter, Display, PartialEq, Eq)]
pub enum Animation {
    Blob(Blob),
    Gradient(Gradient),
    OpposingLines(OpposingLines),
    RandomCircles(RandomCircles),
    RotatingSquare(RotatingSquare),
    Stripes(Stripes),
    Spiral(Spiral),
}

impl Default for Animation {
    fn default() -> Self {
        Self::Gradient(Gradient::default())
    }
}
