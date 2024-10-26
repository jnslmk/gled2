use egui::Ui;

pub mod palette;

pub trait SelectionButton {
    fn selection_button(&mut self, ui: &mut Ui);
}
