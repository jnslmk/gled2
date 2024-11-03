use egui::Ui;

pub mod action;
pub mod asset;
pub mod asset_tree;
pub mod effect;
pub mod group;
pub mod input;
pub mod logo;
pub mod windows;

pub trait ChangeButton {
    fn change_button(&mut self, ui: &mut Ui) -> bool;
}
