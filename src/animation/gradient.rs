use super::{
    config::{CommonConfig, Config},
    renderer::AnimationRenderer,
    Animation,
};
use egui::Ui;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(default)]
pub struct Gradient {
    #[serde(skip)]
    pub renderer: Option<AnimationRenderer>,
    pub config: GradientConfig,
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
            let animation_shader = include_str!("../shaders/gradient.wgsl").to_owned();
            AnimationRenderer::new(&animation_shader, &(&self.config).into())
        });
    }
}

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

impl GradientConfig {
    pub fn ui(&mut self, ui: &mut Ui) {
        self.common.ui(ui);
        ui.label("Todo: gradient");
    }
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
            mode: match config.gradient {
                GradientType::LinearHorizontal => 0,
                GradientType::LinearVertical => 1,
                GradientType::Radial { .. } => 2,
            },
            center: match config.gradient {
                GradientType::Radial { center } => center,
                _ => Default::default(),
            },
            ..Default::default()
        }
    }
}
