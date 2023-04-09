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
pub use state::State;
pub use stripes::{Stripes, StripesConfig};

pub enum Animation {
    Gradient(gradient::Gradient),
    Stripes(stripes::Stripes),
}

// TODO: Macro
impl Animation {
    pub fn renderer(&mut self) -> &AnimationRenderer {
        match self {
            Self::Gradient(gradient) => gradient.renderer(),
            Self::Stripes(stripes) => stripes.renderer(),
        }
    }
}
