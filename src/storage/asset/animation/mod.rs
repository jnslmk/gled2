mod argument;
mod config;
mod renderer;

use super::AssetTrait;
use argument::VariablesCount;
use egui::{Color32, Margin, Stroke, TextureId, Ui};
use egui_extras::syntax_highlighting::CodeTheme;
use serde::{Deserialize, Serialize};

pub use argument::Argument;
pub use config::AnimationConfig;
pub use renderer::AnimationRenderer;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Animation {
    pub shader_code: String,
    pub arguments: Vec<Argument>,
}

impl Default for Animation {
    fn default() -> Self {
        Self {
            shader_code: include_str!("../../../shaders/black.wgsl").to_string(),
            arguments: Default::default(),
        }
    }
}

impl Animation {
    pub fn shader_code_for_getters(&self) -> String {
        let mut count = VariablesCount::default();
        let mut code = String::new();
        for argument in self.arguments.iter() {
            code += &argument.shader_code_for_getter(count);
            count += argument.kind.variables().count();
        }
        code
    }
    pub fn change_shader_code_ui(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;

        let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
            let mut layout_job = egui_extras::syntax_highlighting::highlight(
                ui.ctx(),
                ui.style(),
                &CodeTheme::default(),
                string,
                "rust",
            );
            layout_job.wrap.max_width = wrap_width;
            ui.fonts(|f| f.layout_job(layout_job))
        };

        egui::ScrollArea::vertical().show(ui, |ui| {
            changed = ui
                .add(
                    egui::TextEdit::multiline(&mut self.shader_code)
                        .font(egui::TextStyle::Monospace) // for cursor height
                        .code_editor()
                        .desired_rows(20)
                        .lock_focus(true)
                        .desired_width(f32::INFINITY)
                        .layouter(&mut layouter),
                )
                .changed();
        });

        changed
    }

    pub fn change_arguments_ui(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;
        let mut remove = None;

        let mut count = VariablesCount::default();
        for argument in self.arguments.iter() {
            count += argument.kind.variables().count();
        }
        if !count.possible() {
            ui.vertical_centered_justified(|ui| {
                egui::Frame::none()
                    .inner_margin(Margin::from(3.0))
                    .stroke(Stroke::new(2.0, Color32::RED))
                    .fill(Color32::DARK_RED)
                    .show(ui, |ui| {
                        ui.heading("Too many variables");
                        ui.label(format!("u32: {}/{}", count.u32, 3));
                        ui.label(format!("f32: {}/{}", count.f32, 7));
                    });
            });
        }

        ui.vertical_centered_justified(|ui| {
            if ui.button("+ Add argument").clicked() {
                self.arguments.push(Argument::default());
                changed = true;
            }
        });

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (index, argument) in self.arguments.iter_mut().enumerate() {
                count += argument.kind.variables().count();
                egui::Frame::none()
                    .inner_margin(Margin::from(3.0))
                    .stroke(Stroke::new(1.0, Color32::DARK_GRAY))
                    .show(ui, |ui| {
                        changed |= argument.change_ui(ui, index, &mut remove);
                    });
            }
        });
        if let Some(index) = remove {
            self.arguments.remove(index);
            changed = true;
        }

        changed
    }

    pub fn config_ui(
        &self,
        config: &mut AnimationConfig,
        ui: &mut Ui,
        rendered: TextureId,
    ) -> bool {
        let mut changed = false;
        let mut count = VariablesCount::default();
        for argument in self.arguments.iter() {
            changed |= argument.config_ui(config, ui, count, rendered);
            count += argument.kind.variables().count();
        }

        changed
    }
}

impl AssetTrait for Animation {
    const DIR_NAME: &'static str = "animations";
    const NAME: &'static str = "Animation";
}
