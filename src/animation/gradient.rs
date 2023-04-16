use super::{
    config::{CommonConfig, Config},
    Animation, AnimationConfig,
};
use egui::{CursorIcon, Image, Sense, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Gradient {
    pub common: CommonConfig,
    pub gradient: GradientType,
    pub center: (f32, f32),
}

impl Default for Gradient {
    fn default() -> Self {
        Self {
            common: Default::default(),
            gradient: Default::default(),
            center: (1.0, 1.0),
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

    fn ui(&mut self, ui: &mut egui::Ui, texture_id: egui::TextureId) {
        self.common.ui(ui);
        ui.horizontal(|ui| {
            ui.radio_value(
                &mut self.gradient,
                GradientType::LinearHorizontal,
                "Linear Horizontal",
            );
            ui.radio_value(
                &mut self.gradient,
                GradientType::LinearVertical,
                "Linear Vertical",
            );
            ui.radio_value(&mut self.gradient, GradientType::Radial, "Radial");
        });

        ui.vertical_centered_justified(|ui| {
            if matches!(self.gradient, GradientType::Radial) {
                ui.menu_button("Select center of radial gradient", |ui| {
                    let size = 300.0;
                    let res =
                        ui.add(Image::new(texture_id, Vec2::splat(size)).sense(Sense::click()));
                    if let Some(pos) = res
                        .hover_pos()
                        .map(|pos| pos - res.rect.min)
                        .filter(|pos| pos.x > 0.0 || pos.y > 0.0 || pos.x < size || pos.y < size)
                    {
                        ui.output_mut(|o| o.cursor_icon = CursorIcon::Crosshair);
                        self.center = (pos.x / size, 1.0 - pos.y / size);
                    }
                    if res.clicked() {
                        ui.close_menu();
                    }
                });
            }
        });
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
