use super::{
    config::{CommonConfig, Config},
    Animation, AnimationConfig,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Stripes {
    pub common: CommonConfig,
    pub thickness: f32,
    pub count: u32,
    pub orientation: Orientation,
}

impl AnimationConfig for Stripes {
    fn ui(&mut self, ui: &mut egui::Ui) {
        self.common.ui(ui);
        ui.label("Todo: stripes");
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.orientation, Orientation::Horizontal, "Horizontal");
            ui.radio_value(&mut self.orientation, Orientation::Vertical, "Vertical");
        });
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
