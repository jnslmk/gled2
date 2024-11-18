use crate::{app::PersistantState, ui::ChangeButton, viewport_builder::default_viewport_builder};
use egui::{Context, Id, Vec2, ViewportId};

#[derive(Default)]
pub struct ShortcutsWindow {
    open: bool,
}

impl ShortcutsWindow {
    pub fn update(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }

        ctx.show_viewport_immediate(
            ViewportId(Id::new("shortcuts window")),
            default_viewport_builder()
                .with_title("Gled: Shortcuts")
                .with_inner_size(Vec2::new(500.0, 500.0))
                .with_min_inner_size(Vec2::new(500.0, 500.0)),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.open = false;
                    }
                });

                egui::CentralPanel::default().show(ctx, |ui| {
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
                        persistant_state.tap_input_events =
                            tap_events.into_iter().flatten().collect();
                        persistant_state.freeze_input_events =
                            freeze_events.into_iter().flatten().collect();
                        persistant_state.blackout_input_events =
                            blackout_events.into_iter().flatten().collect();
                        persistant_state.save();
                    }
                });
            },
        );
    }

    pub fn open(&mut self) {
        self.open = true;
    }
}
