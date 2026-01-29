use crate::ui::settings::SettingsEditor;
use crate::ui::window_common::{default_viewport_builder, gled_window_frame};
use egui::{Context, Id, ViewportId};
use emath::Vec2;

pub struct SettingsWindow {
    open: bool,
}

impl Default for SettingsWindow {
    fn default() -> Self {
        Self { open: true }
    }
}

impl SettingsWindow{
    pub fn update(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }
        ctx.show_viewport_immediate(
            ViewportId(Id::new("settings window")),
            default_viewport_builder()
                .with_inner_size(Vec2::new(500.0, 500.0))
                .with_min_inner_size(Vec2::new(500.0, 500.0)),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.open = false;
                    }
                });

                gled_window_frame(ctx, "Settings", |ui| {
                    SettingsEditor::show(ui);
                });
            });
    }
    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }
}