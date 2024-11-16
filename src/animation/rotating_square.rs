use super::{
    config::{CommonConfig, Config},
    set_center::set_center_button,
    Animation, AnimationConfig,
};
use egui::Slider;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct RotatingSquare {
    pub common: CommonConfig,
    pub thickness: f32,
    pub size: f32,
    pub center: (f32, f32),
}

impl Eq for RotatingSquare {}

impl Default for RotatingSquare {
    fn default() -> Self {
        Self {
            common: Default::default(),
            thickness: 0.1,
            size: 0.5,
            center: (0.5, 0.5),
        }
    }
}

impl AnimationConfig for RotatingSquare {
    fn shader_code(&self) -> std::borrow::Cow<str> {
        include_str!("../shaders/rotating_square.wgsl").into()
    }

    fn config(&self) -> Config {
        Config {
            common: self.common,
            thickness: self.thickness,
            size: self.size,
            center: self.center,
            ..Default::default()
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, rendered: egui::TextureId) -> bool {
        let mut changed = false;
        changed |= self.common.ui(ui);
        changed |= ui
            .add(
                Slider::new(&mut self.size, 0.001..=1.0)
                    .text("Size")
                    .custom_formatter(|n, _| format!("{:.1} %", n * 100.0))
                    .custom_parser(|s| s.parse::<f64>().ok().map(|f| f / 100.0)),
            )
            .changed();
        changed |= ui
            .add(
                Slider::new(&mut self.thickness, 0.001..=1.0)
                    .text("Border Thickness")
                    .custom_formatter(|n, _| format!("{:.1} %", n * 100.0))
                    .custom_parser(|s| s.parse::<f64>().ok().map(|f| f / 100.0)),
            )
            .changed();
        changed |= set_center_button(ui, &mut self.center, rendered);
        changed
    }

    fn uses_multiple_colors(&self) -> bool {
        false
    }
}

impl From<RotatingSquare> for Animation {
    fn from(config: RotatingSquare) -> Self {
        Animation::RotatingSquare(config)
    }
}
