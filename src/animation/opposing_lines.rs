use super::{
    config::{CommonConfig, Config},
    Animation, AnimationConfig,
};
use egui::Slider;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct OpposingLines {
    pub common: CommonConfig,
    pub thickness: f32,
    pub orientation: Orientation,
}

impl Eq for OpposingLines {}

impl AnimationConfig for OpposingLines {
    fn ui(&mut self, ui: &mut egui::Ui, _rendered: egui::TextureId) {
        self.common.ui(ui);
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.orientation, Orientation::Horizontal, "Horizontal");
            ui.radio_value(&mut self.orientation, Orientation::Vertical, "Vertical");
        });
        ui.add(
            Slider::new(&mut self.thickness, 0.001..=1.0)
                .text("Thickness")
                .custom_formatter(|n, _| format!("{:.1} %", n * 100.0))
                .custom_parser(|s| s.parse::<f64>().ok().map(|f| f / 100.0)),
        );
    }

    fn shader_code(&self) -> std::borrow::Cow<str> {
        include_str!("../shaders/opposing_lines.wgsl").into()
    }

    fn config(&self) -> Config {
        Config {
            common: self.common,
            thickness: self.thickness,
            mode: match self.orientation {
                Orientation::Horizontal => 0,
                Orientation::Vertical => 1,
            },
            ..Default::default()
        }
    }

    fn uses_multiple_colors(&self) -> bool {
        true
    }
}

impl Default for OpposingLines {
    fn default() -> Self {
        Self {
            common: Default::default(),
            thickness: 0.1,
            orientation: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

impl From<OpposingLines> for Animation {
    fn from(config: OpposingLines) -> Self {
        Animation::OpposingLines(config)
    }
}
