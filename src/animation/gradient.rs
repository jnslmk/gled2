use crate::animation::{config::Direction, renderer::AnimationRenderer, Config};
use wgpu::Device;

use super::Animation;

pub struct Gradient {
    renderer: AnimationRenderer,
    config: GradientConfig,
}

impl Gradient {
    pub fn new(device: &Device, palette: &super::ColorPalette, config: GradientConfig) -> Self {
        let fragment_shader = match config.gradient {
            GradientType::Radial => include_str!("../shaders/gradient_radial.wgsl"),
            GradientType::LinearHorizontal => {
                include_str!("../shaders/gradient_linear_horizontal.wgsl")
            }
            GradientType::LinearVertical => {
                include_str!("../shaders/gradient_linear_vertical.wgsl")
            }
        };
        let renderer = AnimationRenderer::new(device, fragment_shader, palette, &(&config).into());

        Self { renderer, config }
    }

    pub fn renderer(&self) -> &AnimationRenderer {
        &self.renderer
    }

    pub fn config(&self) -> &GradientConfig {
        &self.config
    }
}

impl From<Gradient> for Animation {
    fn from(gradient: Gradient) -> Self {
        Self::Gradient(gradient)
    }
}

pub struct GradientConfig {
    pub direction: Direction,
    pub opacity: f32,
    pub gradient: GradientType,
}

impl Default for GradientConfig {
    fn default() -> Self {
        Self {
            direction: Default::default(),
            opacity: 1.0,
            gradient: Default::default(),
        }
    }
}

#[derive(Default)]
pub enum GradientType {
    #[default]
    Radial,
    LinearHorizontal,
    LinearVertical,
}

impl From<&GradientConfig> for Config {
    fn from(config: &GradientConfig) -> Self {
        Config {
            direction: config.direction,
            opacity: config.opacity,
            ..Default::default()
        }
    }
}
