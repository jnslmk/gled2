//! Renders to a texture

mod colors;
mod config;
mod gradient;
mod renderer;
mod state;
mod stripes;

use self::renderer::AnimationRenderer;

pub use colors::{Color, ColorPalette};
pub use config::{CommonConfig, Direction};
pub use gradient::{Gradient, GradientConfig, GradientType};
pub use renderer::TEXTURE_SIZE;
use serde::{Deserialize, Serialize};
pub use state::State;
pub use stripes::{Stripes, StripesConfig};

#[derive(Serialize, Deserialize, Debug)]
pub enum Animation {
    Gradient(gradient::Gradient),
    Stripes(stripes::Stripes),
}

impl Default for Animation {
    fn default() -> Self {
        Self::Gradient(Gradient::default())
    }
}

// TODO: Macro
impl Animation {
    pub fn renderer(&mut self) -> &AnimationRenderer {
        match self {
            Self::Gradient(gradient) => gradient.renderer(),
            Self::Stripes(stripes) => stripes.renderer(),
        }
    }

    pub fn init_gpu(&mut self) {
        match self {
            Self::Gradient(gradient) => gradient.init_gpu(),
            Self::Stripes(stripes) => stripes.init_gpu(),
        }
    }
}
