use super::{
    config::{CommonConfig, Config},
    Animation, AnimationConfig,
};
use egui::{CursorIcon, Image, Sense, Vec2};
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

    fn ui(&mut self, ui: &mut egui::Ui, texture_id: egui::TextureId) {
        self.common.ui(ui);

        ui.vertical_centered_justified(|ui| {
            ui.style_mut().spacing.interact_size.y = 40.0;
            ui.menu_button("Select center of blob", |ui| {
                let size = 300.0;
                let res = ui.add(Image::new(texture_id, Vec2::splat(size)).sense(Sense::click()));
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
        });
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
