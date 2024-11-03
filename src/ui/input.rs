use super::ChangeButton;
use crate::input::InputEvent;
use egui::{Button, Color32, Ui};

impl ChangeButton for Option<InputEvent> {
    fn change_button(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;

        let mut layout = *ui.layout();
        layout.main_dir = egui::Direction::RightToLeft;

        ui.with_layout(layout, |ui| {
            if self.is_some() && ui.add(Button::new("🗙").fill(Color32::DARK_RED)).clicked() {
                *self = None;
                changed = true;
            }

            layout.main_justify = true;
            ui.with_layout(layout, |ui| {
                ui.menu_button(
                    match self {
                        Some(event) => format!("{event}"),
                        None => "Assign".to_string(),
                    },
                    |ui| {
                        ui.label("Please press a key or provide artnet input!");
                        if let Some(event) = InputEvent::get() {
                            *self = Some(event);
                            ui.close_menu();
                            changed = true;
                        }
                    },
                );
            });
        });

        changed
    }
}
