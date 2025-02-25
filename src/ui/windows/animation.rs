use crate::{
    app::{persistant_state::PersistantState, timing::Timing},
    pipeline::renderer_callback::RendererCallback,
    storage::asset::{
        Asset,
        animation::Animation,
        scene::{effect::Effect, effect_state::EffectState},
    },
    ui::{
        ChangeButton,
        action::UiAction,
        asset_tree::{AssetTree, TREE_WIDTH, TreeSelection},
        effect::widget::EffectWidget,
        viewport_builder::default_viewport_builder,
    },
    wgpu_render_state,
};
use egui::{Color32, Context, Id, Margin, Stroke, Vec2, ViewportId};
use naga::{
    front::wgsl::parse_str,
    valid::{Capabilities, ValidationFlags, Validator},
};
use std::sync::Arc;
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
    copy_rendered_image_to_clipboard: bool,
}

impl AnimationWindow {
    fn validate(&mut self) {
        if let TreeSelection::Asset(animation) = &self.tree.selected() {
            if let Some(effect) = self.effect.as_mut() {
                effect.animation_overwrite = Some(Arc::new(animation.clone()));
                let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
                match parse_str(&effect.shader_code_complete())
                    .map_err(|err| err.emit_to_string(&effect.shader_code_complete()))
                    .and_then(|module| {
                        validator
                            .validate(&module)
                            .map_err(|err| err.emit_to_string(&effect.shader_code_complete()))
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
                &Default::default(),
                1.0,
            );
            let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Render animations for scene editor"),
            });
            effect.render(effect_state, &mut encoder, false);
            if self.copy_rendered_image_to_clipboard {
                self.copy_rendered_image_to_clipboard = false;
                effect_state.copy_rendered_image_to_clipboard();
            }

            RendererCallback::add(encoder.finish());
        }

        if self.preview {
            self.show_preview_window(ctx);
        }

        ctx.show_viewport_immediate(
            ViewportId(Id::new("animations window")),
            default_viewport_builder()
                .with_title("Gled: Animations")
                .with_inner_size(Vec2::new(1000.0, 500.0))
                .with_min_inner_size(Vec2::new(1000.0, 500.0)),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.open = false;
                    }
                });

                egui::SidePanel::left("animations tree")
                    .exact_width(TREE_WIDTH)
                    .resizable(false)
                    .show(ctx, |ui| {
                        if self.tree.show(ui, ui.make_persistent_id("animations_tree")) {
                            self.dirty = false;
                            validate = true;
                            self.effect_state.take();
                        }
                    });

                egui::SidePanel::right("animation editor")
                    .exact_width(300.0)
                    .resizable(false)
                    .show(ctx, |ui| {
                        if let TreeSelection::Asset(animation) = &mut self.tree.selected() {
                            ui.vertical_centered_justified(|ui| {
                                if self.preview {
                                    if ui.button("👁 Close Preview").clicked() {
                                        self.preview = false;
                                    }
                                } else if ui.button("👁 Open Preview").clicked() {
                                    self.preview = true;
                                }
                            });
                            if animation.data.change_arguments_ui(ui) {
                                validate = true;
                                self.dirty = true;
                            }
                        }
                    });

                egui::CentralPanel::default().show(ctx, |ui| {
                    if self.tree.common_settings(ui, &mut self.dirty) {
                        self.effect_state.take();
                        validate = true;
                        if let TreeSelection::Asset(animation) = &self.tree.selected() {
                            UiAction::ReloadShaderCode(animation.id).enqueue();
                        }
                    }

                    if let TreeSelection::Asset(animation) = &mut self.tree.selected() {
                        if let Some(mut error) = self.error.as_deref() {
                            let lines = error.lines().count().max(1);
                            egui::Frame::none()
                                .inner_margin(Margin::from(3.0))
                                .stroke(Stroke::new(2.0, Color32::RED))
                                .fill(Color32::DARK_RED)
                                .show(ui, |ui| {
                                    ui.heading("Error compiling shader code:");
                                    ui.add(
                                        egui::TextEdit::multiline(&mut error)
                                            .font(egui::TextStyle::Monospace)
                                            .code_editor()
                                            .desired_rows(lines)
                                            .desired_width(f32::INFINITY),
                                    )
                                    .context_menu(|ui| {
                                        if ui.button("🖹 Copy error").clicked() {
                                            ui.output_mut(|o| o.copied_text = error.to_owned());
                                            ui.close_menu();
                                        }
                                    });
                                });
                        }

                        if animation.data.change_shader_code_ui(ui) {
                            validate = true;
                            self.dirty = true;
                        }
                    }
                });
            },
        );

        if validate {
            self.validate();
        }
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn show_preview_window(&mut self, ctx: &Context) {
        ctx.show_viewport_immediate(
            ViewportId(Id::new("animation preview window")),
            default_viewport_builder()
                .with_title("Gled: Animation Preview")
                .with_inner_size(Vec2::new(630.0, 400.0))
                .with_min_inner_size(Vec2::new(630.0, 400.0)),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.preview = false;
                    }
                });

                if let (Some(effect), Some(effect_state)) =
                    (&mut self.effect, &mut self.effect_state)
                {
                    egui::SidePanel::right("animation preview right side")
                        .exact_width(300.0)
                        .resizable(false)
                        .show(ctx, |ui| effect.config_ui(effect_state, ui, false));
                }

                egui::CentralPanel::default().show(ctx, |ui| {
                    egui::Frame::none()
                        .inner_margin(Margin::from(6.0))
                        .stroke(Stroke::new(1.0, Color32::DARK_GRAY))
                        .show(ui, |ui| {
                            ui.label("Preview Palette");
                            ui.vertical_centered_justified(|ui| {
                                let mut persistant_state = PersistantState::get();
                                if persistant_state.preview_palette.change_button(ui) {
                                    persistant_state.save();
                                }
                            });
                        });

                    ui.add_space(4.0);

                    if let (Some(effect), Some(effect_state)) =
                        (&mut self.effect, &mut self.effect_state)
                    {
                        egui::Frame::default()
                            .outer_margin(Margin::same(4))
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
                                if ui
                                    .vertical_centered_justified(|ui| {
                                        ui.button("🖹 Copy Image to Clipboard")
                                    })
                                    .inner
                                    .clicked()
                                {
                                    self.copy_rendered_image_to_clipboard = true;
                                }
                            });
                    }
                });
            },
        );
    }
}
