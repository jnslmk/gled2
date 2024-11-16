use super::{
    config::{CommonConfig, Config},
    set_center::set_center_button,
    Animation, AnimationConfig,
};
use egui::Slider;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct Spiral {
    pub common: CommonConfig,
    pub count: u32,
    pub center: (f32, f32),
    pub sharp: bool,
}

impl Eq for Spiral {}

impl Default for Spiral {
    fn default() -> Self {
        Self {
            common: Default::default(),
            count: 1,
            center: (0.5, 0.5),
            sharp: false,
        }
    }
}

impl AnimationConfig for Spiral {
    fn shader_code(&self) -> std::borrow::Cow<str> {
        include_str!("../shaders/spiral.wgsl").into()
    }

    fn config(&self) -> Config {
        Config {
            common: self.common,
            count: self.count,
            center: self.center,
            mode: if self.sharp { 1 } else { 0 },
            ..Default::default()
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, rendered: egui::TextureId) -> bool {
        let mut changed = false;
        changed |= self.common.ui(ui);
        changed |= ui.checkbox(&mut self.sharp, "Sharp").changed();
        changed |= ui
            .add(Slider::new(&mut self.count, 1..=100).text("Count"))
            .changed();
        changed |= set_center_button(ui, &mut self.center, rendered);
        changed
    }

    fn uses_multiple_colors(&self) -> bool {
        false
    }
}

impl From<Spiral> for Animation {
    fn from(config: Spiral) -> Self {
        Animation::Spiral(config)
    }
}
