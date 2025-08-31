use crate::{
    storage::asset::project::Project,
    ui::{
        ChangeButton,
        window_common::{default_viewport_builder, gled_window_frame},
    },
};
use egui::{
    Context, Id, ScrollArea, Vec2, ViewportId, scroll_area::ScrollBarVisibility::AlwaysVisible,
};

#[derive(Default)]
pub struct ShortcutsWindow {
    open: bool,
}

impl ShortcutsWindow {
    pub fn update(&mut self, ctx: &Context, project: Option<&mut Project>) {
        if !self.open {
            return;
        }
        let Some(project) = project else {
            return;
        };

        ctx.show_viewport_immediate(
            ViewportId(Id::new("shortcuts window")),
            default_viewport_builder()
                .with_inner_size(Vec2::new(500.0, 500.0))
                .with_min_inner_size(Vec2::new(500.0, 500.0)),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.open = false;
                    }
                });

                gled_window_frame(ctx, "Shortcuts", |ui| {
                    ScrollArea::vertical()
                        .scroll_bar_visibility(AlwaysVisible)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());

                            let mut changed = false;
                            let mut tap_events = project
                                .tap_input_events
                                .iter()
                                .cloned()
                                .map(Option::Some)
                                .chain(std::iter::once(None))
                                .collect::<Vec<_>>();
                            let mut blackout_events = project
                                .blackout_input_events
                                .iter()
                                .cloned()
                                .map(Option::Some)
                                .chain(std::iter::once(None))
                                .collect::<Vec<_>>();
                            let mut blackout_hold_events = project
                                .blackout_hold_input_events
                                .iter()
                                .cloned()
                                .map(Option::Some)
                                .chain(std::iter::once(None))
                                .collect::<Vec<_>>();
                            let mut half_events = project
                                .half_input_events
                                .iter()
                                .cloned()
                                .map(Option::Some)
                                .chain(std::iter::once(None))
                                .collect::<Vec<_>>();
                            let mut double_events = project
                                .double_input_events
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
                            ui.heading("Blackout Toggle");
                            for event in blackout_events.iter_mut() {
                                changed |= event.change_button(ui);
                            }
                            ui.separator();
                            ui.heading("Blackout Hold");
                            for event in blackout_hold_events.iter_mut() {
                                changed |= event.change_button(ui);
                            }
                            ui.separator();
                            ui.heading("Half the speed");
                            for event in half_events.iter_mut() {
                                changed |= event.change_button(ui);
                            }
                            ui.separator();
                            ui.heading("Double the speed");
                            for event in double_events.iter_mut() {
                                changed |= event.change_button(ui);
                            }

                            if changed {
                                project.tap_input_events =
                                    tap_events.into_iter().flatten().collect();
                                project.blackout_input_events =
                                    blackout_events.into_iter().flatten().collect();
                                project.half_input_events =
                                    half_events.into_iter().flatten().collect();
                                project.double_input_events =
                                    double_events.into_iter().flatten().collect();
                            }
                        });
                });
            },
        );
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }
}
