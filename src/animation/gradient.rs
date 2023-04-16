use super::{
    config::{CommonConfig, Config},
    Animation, AnimationConfig,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct Gradient {
    pub common: CommonConfig,
    pub gradient: GradientType,
}

impl AnimationConfig for Gradient {
    fn shader_code(&self) -> std::borrow::Cow<str> {
        include_str!("../shaders/gradient.wgsl").into()
    }

    fn config(&self) -> Config {
        Config {
            common: self.common,
            mode: match self.gradient {
                GradientType::LinearHorizontal => 0,
                GradientType::LinearVertical => 1,
                GradientType::Radial { .. } => 2,
            },
            center: match self.gradient {
                GradientType::Radial { center } => center,
                _ => Default::default(),
            },
            ..Default::default()
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
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

impl From<Gradient> for Animation {
    fn from(config: Gradient) -> Self {
        Animation::Gradient(config)
    }
}
