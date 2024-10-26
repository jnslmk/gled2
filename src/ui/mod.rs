use egui::Ui;

pub mod action;
pub mod effect;
pub mod group;
pub mod input;
pub mod palette;
pub mod text_input;
pub mod windows;

pub trait ChangeButton {
    fn change_button(&mut self, ui: &mut Ui);
}
