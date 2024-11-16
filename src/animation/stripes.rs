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
    fn ui(&mut self, ui: &mut egui::Ui, _rendered: egui::TextureId) -> bool {
        let mut changed = false;
        changed |= self.common.ui(ui);

        ui.horizontal(|ui| {
            changed |= ui
                .radio_value(&mut self.orientation, Orientation::Horizontal, "Horizontal")
                .changed();
            changed |= ui
                .radio_value(&mut self.orientation, Orientation::Vertical, "Vertical")
                .changed();
        });

        ui.label("Thickness");
        ui.vertical_centered_justified(|ui| {
            changed |= ui
                .add(
                    Slider::new(&mut self.thickness, 0.001..=1.0)
                        .custom_formatter(|n, _| format!("{:.1} %", n * 100.0))
                        .custom_parser(|s| s.parse::<f64>().ok().map(|f| f / 100.0)),
                )
                .changed();
        });

        ui.label("Bars");
        ui.vertical_centered_justified(|ui| {
            changed |= ui.add(Slider::new(&mut self.count, 1..=15)).changed();
        });

        changed
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

    fn uses_multiple_colors(&self) -> bool {
        true
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
