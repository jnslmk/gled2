use crate::ui::ChangeButton;
use egui::Ui;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Clone, Copy)]
#[serde(transparent)]
pub struct ColorShift(pub f32);

impl Eq for ColorShift {}
impl ChangeButton for ColorShift {
    fn change_button(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;

        let response = ui.add(egui::Slider::new(&mut self.0, 0.0..=360.0));
        if response.changed() {
            changed = true;
        }

        changed
    }
}
