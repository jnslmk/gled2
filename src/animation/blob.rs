use super::{set_center::set_center_button, Animation, AnimationConfig};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct Blob {
    pub common: CommonConfig,
    pub center: (f32, f32),
}

impl Eq for Blob {}

impl Default for Blob {
    fn default() -> Self {
        Self {
            common: Default::default(),
            center: (0.5, 0.5),
        }
    }
}

impl AnimationConfig for Blob {
    fn shader_code(&self) -> std::borrow::Cow<str> {
        include_str!("../shaders/blob.wgsl").into()
    }

    fn config(&self) -> AnimationConfig {
        Config {
            common: self.common,
            center: self.center,
            ..Default::default()
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, rendered: egui::TextureId) -> bool {
        let mut changed = self.common.ui(ui);
        changed |= set_center_button(ui, &mut self.center, rendered);
        changed
    }
}

impl From<Blob> for Animation {
    fn from(config: Blob) -> Self {
        Animation::Blob(config)
    }
}
