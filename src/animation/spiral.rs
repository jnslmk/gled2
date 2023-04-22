use super::{
    config::{CommonConfig, Config},
    Animation, AnimationConfig,
};
use egui::{CursorIcon, Image, Sense, Slider, Vec2};
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

    fn ui(&mut self, ui: &mut egui::Ui, texture_id: egui::TextureId) {
        self.common.ui(ui);

        ui.checkbox(&mut self.sharp, "Sharp");
        ui.add(Slider::new(&mut self.count, 1..=100).text("Count"));

        ui.vertical_centered_justified(|ui| {
            ui.style_mut().spacing.interact_size.y = 40.0;
            ui.menu_button("Select center of spiral", |ui| {
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
}

impl From<Spiral> for Animation {
    fn from(config: Spiral) -> Self {
        Animation::Spiral(config)
    }
}
