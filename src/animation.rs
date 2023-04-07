//! Renders to a texture

mod colors;
mod config;
mod gradient;
mod renderer;
mod state;

use wgpu::{CommandEncoder, Queue, Texture};

pub use colors::{Color, ColorPalette};
pub use config::Config;
pub use gradient::{Gradient, GradientConfig, GradientType};

pub enum Animation {
    Gradient(gradient::Gradient),
}

impl Animation {
    pub fn texture(&self) -> &Texture {
        match self {
            Self::Gradient(gradient) => gradient.renderer().texture(),
        }
    }

    pub fn prepare(&self, queue: &Queue) {
        match self {
            Self::Gradient(gradient) => gradient.renderer().prepare(queue),
        }
    }

    pub fn render(&self, encoder: &mut CommandEncoder) {
        match self {
            Self::Gradient(gradient) => gradient.renderer().render(encoder),
        }
    }
}
