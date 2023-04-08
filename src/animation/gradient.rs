use super::{
    config::{CommonConfig, Config},
    renderer::AnimationRenderer,
    Animation,
};
use wgpu::Device;

pub struct Gradient {
    renderer: AnimationRenderer,
    config: GradientConfig,
}

impl Gradient {
    pub fn new(device: &Device, palette: &super::ColorPalette, config: GradientConfig) -> Self {
        let mut animation_shader = include_str!("../shaders/gradient_common.wgsl").to_owned();
        animation_shader.push_str(match config.gradient {
            GradientType::Radial { .. } => include_str!("../shaders/gradient_radial.wgsl"),
            GradientType::LinearHorizontal => {
                include_str!("../shaders/gradient_linear_horizontal.wgsl")
            }
            GradientType::LinearVertical => {
                include_str!("../shaders/gradient_linear_vertical.wgsl")
            }
        });
        let renderer =
            AnimationRenderer::new(device, &animation_shader, palette, &(&config).into());

        Self { renderer, config }
    }

    pub fn renderer(&self) -> &AnimationRenderer {
        &self.renderer
    }

    pub fn config(&self) -> &GradientConfig {
        &self.config
    }
}

//TODO: Macro
impl From<Gradient> for Animation {
    fn from(gradient: Gradient) -> Self {
        Self::Gradient(gradient)
    }
}

#[derive(Default, Debug)]
pub struct GradientConfig {
    pub common: CommonConfig,
    pub gradient: GradientType,
}

#[derive(Debug)]
pub enum GradientType {
    Radial { center: (f32, f32) },
    LinearHorizontal,
    LinearVertical,
}

impl Default for GradientType {
    fn default() -> Self {
        Self::Radial { center: (0.5, 0.5) }
    }
}

impl From<&GradientConfig> for Config {
    fn from(config: &GradientConfig) -> Self {
        Config {
            common: config.common,
            center: match config.gradient {
                GradientType::Radial { center } => center,
                _ => Default::default(),
            },
            ..Default::default()
        }
    }
}
