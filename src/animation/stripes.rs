use super::{
    config::{CommonConfig, Config},
    Animation, AnimationConfig,
};
use egui::Slider;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct Stripes {
    pub common: CommonConfig,
    pub thickness: f32,
    pub count: u32,
    pub orientation: Orientation,
}

impl Eq for Stripes {}

impl AnimationConfig for Stripes {
    fn ui(&mut self, ui: &mut egui::Ui, _texture_id: egui::TextureId) {
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
        ui.add(Slider::new(&mut self.count, 1..=15).text("Bars"));
    }

    fn shader_code(&self) -> std::borrow::Cow<str> {
        include_str!("../shaders/stripes.wgsl").into()
    }

    fn config(&self) -> Config {
        Config {
            common: self.common,
            thickness: self.thickness,
            count: self.count,
            mode: match self.orientation {
                Orientation::Horizontal => 0,
                Orientation::Vertical => 1,
            },
            ..Default::default()
        }
    }
}

impl Default for Stripes {
    fn default() -> Self {
        Self {
            common: Default::default(),
            thickness: 0.1,
            count: 5,
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

impl From<Stripes> for Animation {
    fn from(config: Stripes) -> Self {
        Animation::Stripes(config)
    }
}
