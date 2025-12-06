use egui::{Color32, Stroke, Ui};

pub mod action;
pub mod asset;
pub mod asset_tree;
pub mod brightness_slider;
pub mod effect;
pub mod input;
pub mod logo;
pub mod pills;
pub mod scene_instance;
pub mod temperature;
pub mod update_check;
pub mod window_common;
pub mod windows;

pub static FRAME_STROKE: Stroke = Stroke {
    width: 0.3,
    color: Color32::WHITE,
};

pub trait ChangeButton {
    fn change_button(&mut self, ui: &mut Ui) -> bool;
}

impl ChangeButton for f32 {
    fn change_button(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;

        let response = ui.add(egui::Slider::new(self, 0.0..=1.0));
        if response.changed() {
            changed = true;
        }

        changed
    }
}

pub trait OverwriteChangeButton: Default + ChangeButton {
    const NAME: &'static str;
}

// for overwrites
impl<T: OverwriteChangeButton> ChangeButton for Option<T> {
    fn change_button(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;

        let mut overwrite = self.is_some();
        ui.checkbox(&mut overwrite, format!("Overwrite {}", T::NAME));
        if self.is_none() && overwrite {
            *self = Some(T::default());
            changed = true;
        } else if self.is_some() && !overwrite {
            *self = None;
            changed = true;
        }
        if let Some(asset_id) = self {
            ui.vertical_centered_justified(|ui| {
                changed |= asset_id.change_button(ui);
            });
        }

        changed
    }
}
