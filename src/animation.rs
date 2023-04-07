//! Renders to a texture

mod colors;
mod config;
mod gradient;
mod renderer;
mod state;
mod stripes;

use self::renderer::AnimationRenderer;

pub use colors::{Color, ColorPalette};
pub use config::{Config, Direction};
pub use gradient::{Gradient, GradientConfig, GradientType};
pub use stripes::{Stripes, StripesConfig};

pub enum Animation {
    Gradient(gradient::Gradient),
    Stripes(stripes::Stripes),
}

// TODO: Macro
impl Animation {
    pub fn renderer(&self) -> &AnimationRenderer {
        match self {
            Self::Gradient(gradient) => gradient.renderer(),
            Self::Stripes(stripes) => stripes.renderer(),
        }
    }
}
