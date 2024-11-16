use super::{
    config::{CommonConfig, Config},
    set_center::set_center_button,
    Animation, AnimationConfig,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct Gradient {
    pub common: CommonConfig,
    pub gradient: GradientType,
    pub center: (f32, f32),
}

impl Eq for Gradient {}

impl Default for Gradient {
    fn default() -> Self {
        Self {
            common: Default::default(),
            gradient: Default::default(),
            center: (0.5, 0.5),
        }
    }
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
            center: self.center,
            ..Default::default()
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, rendered: egui::TextureId) -> bool {
        let mut changed = false;
        changed |= self.common.ui(ui);
        ui.horizontal(|ui| {
            changed |= ui
                .radio_value(
                    &mut self.gradient,
                    GradientType::LinearHorizontal,
                    "Linear Horizontal",
                )
                .changed();
            changed |= ui
                .radio_value(
                    &mut self.gradient,
                    GradientType::LinearVertical,
                    "Linear Vertical",
                )
                .changed();
            changed |= ui
                .radio_value(&mut self.gradient, GradientType::Radial, "Radial")
                .changed();
        });

        if matches!(self.gradient, GradientType::Radial) {
            changed |= set_center_button(ui, &mut self.center, rendered);
        }

        changed
    }

    fn uses_multiple_colors(&self) -> bool {
        true
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum GradientType {
    Radial,
    LinearHorizontal,
    LinearVertical,
}

impl Default for GradientType {
    fn default() -> Self {
        Self::Radial
    }
}

impl From<Gradient> for Animation {
    fn from(config: Gradient) -> Self {
        Animation::Gradient(config)
    }
}
