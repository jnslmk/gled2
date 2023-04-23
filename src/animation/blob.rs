use super::{
    config::{CommonConfig, Config},
    set_center::set_center_button,
    Animation, AnimationConfig,
};
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

    fn config(&self) -> Config {
        Config {
            common: self.common,
            center: self.center,
            ..Default::default()
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, rendered: egui::TextureId, svg: Option<egui::TextureId>) {
        self.common.ui(ui);
        set_center_button(ui, &mut self.center, rendered, svg);
    }

    fn uses_multiple_colors(&self) -> bool {
        false
    }
}

impl From<Blob> for Animation {
    fn from(config: Blob) -> Self {
        Animation::Blob(config)
    }
}
