//! Renders to a texture

mod colors;
mod config;
mod gradient;
mod renderer;
mod state;
mod stripes;

use gled_proc_macros::Animation;
use renderer::AnimationRenderer;
use serde::{Deserialize, Serialize};

pub use colors::{Color, ColorPalette};
pub use config::{CommonConfig, Direction};
pub use gradient::{Gradient, GradientConfig, GradientType};
pub use state::State;
pub use stripes::{Stripes, StripesConfig};

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
