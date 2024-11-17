use crate::{
    app::{PersistantState, Timing},
    effect::{Effect, EffectState},
    group::Groups,
    storage::{Animation, Asset},
    ui::{
        action::Action,
        asset_tree::{AssetTree, TreeSelection, TREE_WIDTH},
        effect::EffectWidget,
    },
    wgpu_render_state,
};
use egui::{Color32, Context, Margin, Stroke, Vec2};
use naga::{
    front::wgsl::parse_str,
    valid::{Capabilities, ValidationFlags, Validator},
};
use std::iter::once;
use wgpu::CommandEncoderDescriptor;

#[derive(Default)]
pub struct AnimationWindow {
    open: bool,
    dirty: bool,
    tree: AssetTree<Animation>,
    effect: Option<Effect>,
    effect_state: Option<EffectState>,
    error: Option<String>,
    preview: bool,
}

impl AnimationWindow {
    fn validate(&mut self) {
        if let TreeSelection::Asset(animation) = &self.tree.selected() {
            if let Some(effect) = self.effect.as_mut() {
                effect.animation_overwrite = Some(animation.clone());
                let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
                match parse_str(&effect.shader_code())
                    .map_err(|err| err.emit_to_string(&effect.shader_code()))
                    .and_then(|module| {
                        validator
                            .validate(&module)
                            .map_err(|err| err.emit_to_string(&effect.shader_code()))
                    }) {
                    Ok(_) => {
                        if let Some(effect_state) = self.effect_state.as_mut() {
                            effect_state.update(effect);
                        } else {
                            self.effect_state = Some(EffectState::new(effect));
                        }
                        self.error.take();
                    }
                    Err(err) => {
                        self.error = Some(err);
                    }
                }
            }
        }
    }

    pub fn update(&mut self, ctx: &Context, timing: &Timing) {
        if !self.open {
            self.dirty = false;
            return;
        }

        let mut validate = false;

        if let TreeSelection::Asset(..) = &self.tree.selected() {
            if self.effect.is_none() {
                self.effect = Some(Default::default());
                validate = true;
            }
            if self.effect_state.is_none() {
                validate = true;
            }
        }

        if let (Some(effect), Some(effect_state)) = (&mut self.effect, &mut self.effect_state) {
            effect_state.beat_progression = timing.beat_progression();
            effect_state.beats_per_minute = timing.beats_per_minute;
            effect_state.framerate = timing.framerate().unwrap_or_default();
            let wgpu_render_state = wgpu_render_state();
            let device = wgpu_render_state.device;
            let queue = &wgpu_render_state.queue;
            effect.prepare(
                effect_state,
                queue,
                PersistantState::get().preview_palette.and_then(Asset::get),
                &Groups::None,
                1.0,
            );
            let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Render animations for scene editor"),
            });
            effect.render(effect_state, &mut encoder, false);
            queue.submit(once(encoder.finish()));
        }

        if self.preview {
            self.show_preview_window(ctx);
        }

        egui::Window::new("Animations")
            .collapsible(false)
            .min_width(1000.0)
            .resizable(true)
            .default_pos(ctx.available_rect().center())
            .open(&mut self.open)
            .show(ctx, |ui| {
                egui::SidePanel::left("animations tree")
                    .exact_width(TREE_WIDTH)
                    .resizable(false)
                    .show_inside(ui, |ui| {
                        if self.tree.show(ui, ui.make_persistent_id("animations_tree")) {
                            self.dirty = false;
                        }
                    });

                egui::SidePanel::right("animation editor")
                    .exact_width(300.0)
                    .resizable(false)
                    .show_inside(ui, |ui| {
                        if let TreeSelection::Asset(animation) = &mut self.tree.selected() {
                            ui.vertical_centered_justified(|ui| {
                                if ui.button("👁 Open Preview").clicked() {
                                    self.preview = true;
                                }
                            });
                            self.dirty |= animation.data.change_arguments_ui(ui);
                        }
                    });

                egui::Frame::default()
                    .outer_margin(Margin::same(4.0))
                    .show(ui, |ui| {
                        if self.tree.common_settings(ui, &mut self.dirty) {
                            self.effect_state.take();
                            self.error.take();
                            if let TreeSelection::Asset(animation) = &self.tree.selected() {
                                Action::ReloadShaderCode(animation.id).enqueue();
                            }
                        }

                        if let TreeSelection::Asset(animation) = &mut self.tree.selected() {
                            if animation.data.change_shader_code_ui(ui) {
                                validate = true;
                                self.dirty = true;
                            }

                            if let Some(error) = self.error.as_mut() {
                                let lines = error.lines().count().max(1);
                                ui.vertical_centered_justified(|ui| {
                                    egui::Frame::none()
                                        .inner_margin(Margin::from(3.0))
                                        .stroke(Stroke::new(2.0, Color32::RED))
                                        .fill(Color32::DARK_RED)
                                        .show(ui, |ui| {
                                            ui.heading("Error compiling shader code:");
                                            ui.add_enabled(
                                                true,
                                                egui::TextEdit::multiline(error)
                                                    .font(egui::TextStyle::Monospace)
                                                    .code_editor()
                                                    .desired_rows(lines)
                                                    .lock_focus(true)
                                                    .desired_width(f32::INFINITY),
                                            );
                                        });
                                });
                            }
                        }
                    });
            });

        if validate {
            self.validate();
        }
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn show_preview_window(&mut self, ctx: &Context) {
        egui::Window::new("Animation Preview")
            .collapsible(false)
            .min_width(600.0)
            .resizable(true)
            .default_pos(ctx.available_rect().center())
            .open(&mut self.preview)
            .show(ctx, |ui| {
                if let (Some(effect), Some(effect_state)) =
                    (&mut self.effect, &mut self.effect_state)
                {
                    egui::SidePanel::right("animation preview right side")
                        .exact_width(300.0)
                        .resizable(false)
                        .show_inside(ui, |ui| effect.config_ui(effect_state, ui, false));

                    egui::Frame::default()
                        .outer_margin(Margin::same(4.0))
                        .show(ui, |ui| {
                            ui.add_sized(
                                Vec2::splat(300.0),
                                EffectWidget {
                                    show_group: false,
                                    selectable: None,
                                    effect,
                                    effect_state,
                                },
                            );
                        });
                }
            });
    }
}
