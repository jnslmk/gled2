use serde::{Deserialize, Serialize};

use crate::constants::GPU_NOT_INIT;

use super::{
    config::{CommonConfig, Config},
    renderer::AnimationRenderer,
    Animation,
};

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(default)]
pub struct Gradient {
    #[serde(skip)]
    renderer: Option<AnimationRenderer>,
    config: GradientConfig,
}

impl Clone for Gradient {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            ..Default::default()
        }
    }
}

impl Gradient {
    pub fn new(config: GradientConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    pub fn init_gpu(&mut self) {
        self.renderer.get_or_insert_with(|| {
            let mut animation_shader = include_str!("../shaders/gradient_common.wgsl").to_owned();
            animation_shader.push_str(match self.config.gradient {
                GradientType::Radial { .. } => include_str!("../shaders/gradient_radial.wgsl"),
                GradientType::LinearHorizontal => {
                    include_str!("../shaders/gradient_linear_horizontal.wgsl")
                }
                GradientType::LinearVertical => {
                    include_str!("../shaders/gradient_linear_vertical.wgsl")
                }
            });

            AnimationRenderer::new(&animation_shader, &(&self.config).into())
        });
    }

    pub fn renderer(&mut self) -> &AnimationRenderer {
        self.renderer.as_ref().expect(GPU_NOT_INIT)
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

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct GradientConfig {
    pub common: CommonConfig,
    pub gradient: GradientType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
