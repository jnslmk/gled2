use crate::audio::adsr_editor::ADSREditor;
use crate::ui::window_common::{default_viewport_builder, gled_window_frame};
use egui::{Context, Id, ViewportId};
use emath::Vec2;

pub struct ADSREditorWindow {
    pub open: bool,
    pub editor: ADSREditor,
}
impl Default for ADSREditorWindow {
    fn default() -> Self {
        let editor = ADSREditor::default();
        Self {
            open: true,
            editor,
        }
    }
}

impl ADSREditorWindow {
    pub fn update(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }
        // update: reactive audio thread -> ui copy of adsr params
        ctx.show_viewport_immediate(
            ViewportId(Id::new("ADSR Editor")),
            default_viewport_builder()
                .with_inner_size(Vec2::new(500., 700.))
                .with_min_inner_size(Vec2::new(500.0, 700.)),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.open = false;
                    }
                });
                gled_window_frame(ctx, "ADSR Editor", |ui| {
                    egui::CentralPanel::default().show_inside(ui, |ui| {
                        self.editor.show(ui);
                    });
                });
            },
        );
    }
}
