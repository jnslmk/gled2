use super::{
    config::{CommonConfig, Config},
    Animation, AnimationConfig,
};
use egui::Slider;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct RandomCircles {
    pub common: CommonConfig,
    pub count: u32,
}

impl Eq for RandomCircles {}

impl AnimationConfig for RandomCircles {
    fn ui(&mut self, ui: &mut egui::Ui, _texture_id: egui::TextureId) {
        self.common.ui(ui);
        ui.add(Slider::new(&mut self.count, 1..=10).text("Density"));
    }

    fn shader_code(&self) -> std::borrow::Cow<str> {
        include_str!("../shaders/random_circles.wgsl").into()
    }

    fn config(&self) -> Config {
        Config {
            common: self.common,
            count: self.count,
            ..Default::default()
        }
    }

    fn uses_multiple_colors(&self) -> bool {
        true
    }
}

impl Default for RandomCircles {
    fn default() -> Self {
        Self {
            common: Default::default(),
            count: 5,
        }
    }
}

impl From<RandomCircles> for Animation {
    fn from(config: RandomCircles) -> Self {
        Animation::RandomCircles(config)
    }
}
