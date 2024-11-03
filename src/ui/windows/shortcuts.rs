use crate::{app::PersistantState, ui::ChangeButton};
use egui::Context;

#[derive(Default)]
pub struct ShortcutsWindow {
    pub open: bool,
}

impl ShortcutsWindow {
    pub fn update(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }

        egui::Window::new("Shortcuts")
            .collapsible(false)
            .min_width(500.0)
            .resizable(true)
            .default_pos(ctx.available_rect().center())
            .open(&mut self.open)
            .show(ctx, |ui| {
                let mut changed = false;

                let mut persistant_state = PersistantState::get();
                let mut tap_events = persistant_state
                    .tap_input_events
                    .iter()
                    .cloned()
                    .map(Option::Some)
                    .chain(std::iter::once(None))
                    .collect::<Vec<_>>();
                let mut freeze_events = persistant_state
                    .freeze_input_events
                    .iter()
                    .cloned()
                    .map(Option::Some)
                    .chain(std::iter::once(None))
                    .collect::<Vec<_>>();
                let mut blackout_events = persistant_state
                    .blackout_input_events
                    .iter()
                    .cloned()
                    .map(Option::Some)
                    .chain(std::iter::once(None))
                    .collect::<Vec<_>>();

                ui.heading("Tap");
                for event in tap_events.iter_mut() {
                    changed |= event.change_button(ui);
                }
                ui.separator();
                ui.heading("Freeze");
                for event in freeze_events.iter_mut() {
                    changed |= event.change_button(ui);
                }
                ui.separator();
                ui.heading("Blackout");
                for event in blackout_events.iter_mut() {
                    changed |= event.change_button(ui);
                }

                if changed {
                    persistant_state.tap_input_events = tap_events.into_iter().flatten().collect();
                    persistant_state.freeze_input_events =
                        freeze_events.into_iter().flatten().collect();
                    persistant_state.blackout_input_events =
                        blackout_events.into_iter().flatten().collect();
                    persistant_state.save();
                }
            });
    }

    pub fn open(&mut self) {
        self.open = true;
    }
}
