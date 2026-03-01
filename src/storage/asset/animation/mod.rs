pub mod argument;
pub mod config;
pub mod renderer;

use crate::{audio::sound_trigger_data::SoundTriggerData, storage::collections::Collections};

use super::AssetTrait;
use argument::{Argument, variables::VariablesCount};
use config::AnimationConfig;
use egui::{
    Color32, Margin, Stroke, TextureId, Ui, UiKind, scroll_area::ScrollBarVisibility::AlwaysVisible,
};
use egui_extras::syntax_highlighting::CodeTheme;
use serde::{Deserialize, Serialize};

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

        let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
            let mut layout_job = egui_extras::syntax_highlighting::highlight(
                ui.ctx(),
                ui.style(),
                &CodeTheme::default(),
                buf.as_str(),
                "rust",
            );
            layout_job.wrap.max_width = wrap_width;
            ui.fonts_mut(|f| f.layout_job(layout_job))
        };

        ui.with_layout(
            egui::Layout::left_to_right(egui::Align::Min).with_cross_justify(true),
            |ui| {
                egui::ScrollArea::vertical()
                    .scroll_bar_visibility(AlwaysVisible)
                    .show(ui, |ui| {
                        let response = ui.add(
                            egui::TextEdit::multiline(&mut self.shader_code)
                                .id_salt("Code Editor")
                                .font(egui::TextStyle::Monospace) // for cursor height
                                .code_editor()
                                .desired_rows(20)
                                .lock_focus(true)
                                .desired_width(f32::INFINITY)
                                .layouter(&mut layouter),
                        );
                        response.context_menu(|ui| {
                            if ui.button("🖹 Copy whole source code").clicked() {
                                ui.ctx().copy_text(self.shader_code_complete());
                                ui.close_kind(UiKind::Menu);
                            }
                        });
                        changed = response.changed();
                    });
            },
        );

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
                egui::Frame::NONE
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

        egui::ScrollArea::vertical()
            .scroll_bar_visibility(AlwaysVisible)
            .show(ui, |ui| {
                for (index, argument) in self.arguments.iter_mut().enumerate() {
                    count += argument.kind.variables().count();
                    egui::Frame::NONE
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
        svg: Option<egui::TextureHandle>,
        beat_progression: f32,
        collections: &mut Collections,
        sound_trigger_data: &SoundTriggerData,
    ) -> bool {
        let mut changed = false;
        let mut count = VariablesCount::default();
        for argument in self.arguments.iter() {
            changed |= argument.config_ui(
                config,
                ui,
                count,
                rendered,
                svg.clone(),
                beat_progression,
                collections,
                sound_trigger_data,
            );
            count += argument.kind.variables().count();
        }

        changed
    }

    /// Complete shader code with common.wgsl and getters for arguments
    pub fn shader_code_complete(&self) -> String {
        format!(
            "{}\n\n{}\n\n/**************************************************\n        Do not edit/paste code above this line!\n**************************************************/\n\n{}",
            include_str!("../../../shaders/common.wgsl"),
            self.shader_code_for_getters(),
            self.shader_code
        )
    }
}

impl AssetTrait for Animation {
    const DIR_NAME: &'static str = "animations";
    const NAME: &'static str = "Animation";
    const SHOW_NAME_IF_SELECTED: bool = true;
}
