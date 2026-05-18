use crate::{
    app::{persistent_state::PersistentState, timing::Timing},
    audio::sound_data::SoundData,
    pipeline::renderer_callback::RendererCallback,
    storage::{
        asset::{Asset, animation::Animation, scene::effect::Effect},
        collections::Collections,
    },
    ui::{
        action::UiAction,
        asset::CollectionsChangeButton,
        asset_tree::{AssetTree, TREE_WIDTH, TreeSelection},
        effect::widget::EffectWidget,
        window_common::{default_viewport_builder, gled_window_frame},
    },
    wgpu_render_state,
};
use egui::{
    Color32, Context, Id, Margin, ScrollArea, Stroke, UiKind, Vec2, ViewportId,
    scroll_area::ScrollBarVisibility::AlwaysVisible,
};
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
    error: Option<String>,
    preview: bool,
    copy_rendered_image_to_clipboard: bool,
}

impl AnimationWindow {
    fn validate(&mut self, collections: &Collections) {
        if let TreeSelection::Asset(animation) = &self.tree.selected()
            && let Some(effect) = self.effect.as_mut()
        {
            effect.animation_overwrite = Some(Arc::new(animation.clone()));
            let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
            match parse_str(&effect.shader_code_complete(collections))
                .map_err(|err| err.emit_to_string(&effect.shader_code_complete(collections)))
                .and_then(|module| {
                    validator.validate(&module).map_err(|err| {
                        err.emit_to_string(&effect.shader_code_complete(collections))
                    })
                }) {
                Ok(_) => {
                    effect
                        .state
                        .set_shader_code(&effect.shader_code_complete(collections));
                    self.error.take();
                }
                Err(err) => {
                    self.error = Some(err);
                }
            }
        }
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn update(
        &mut self,
        ctx: &Context,
        timing: &Timing,
        collections: &mut Collections,
        persistent_state: &mut PersistentState,
        sound_data: &mut SoundData,
    ) {
        if !self.open {
            self.dirty = false;
            return;
        }

        let mut validate = false;

        if let TreeSelection::Asset(..) = &self.tree.selected()
            && self.effect.is_none()
        {
            self.effect = Some(Default::default());
            validate = true;
        }

        if let Some(effect) = &mut self.effect {
            effect.state.beat_progression = timing.beat_progression();
            effect.state.beats_per_minute = timing.beats_per_minute();
            effect.state.framerate = timing.framerate().unwrap_or_default();
            let wgpu_render_state = wgpu_render_state();
            let device = wgpu_render_state.device;
            let queue = &wgpu_render_state.queue;
            effect.prepare(
                queue,
                persistent_state
                    .preview_palette()
                    .and_then(|id| Asset::get(id, collections))
                    .map(|palette| palette.data.clone()),
                &Default::default(),
                1.0,
                collections,
                sound_data,
            );
            let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Render animations for animation editor"),
            });
            effect.render(&mut encoder, false);
            if self.copy_rendered_image_to_clipboard {
                self.copy_rendered_image_to_clipboard = false;
                effect.state.copy_rendered_image_to_clipboard();
            }

            RendererCallback::add(encoder.finish());
        }

        if self.preview {
            self.show_preview_window(ctx, collections, persistent_state, sound_data);
        }

        ctx.show_viewport_immediate(
            ViewportId(Id::new("animations window")),
            default_viewport_builder()
                .with_inner_size(Vec2::new(1000.0, 540.0))
                .with_min_inner_size(Vec2::new(1000.0, 540.0)),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.open = false;
                    }
                });

                gled_window_frame(ctx, "Animations", |ui| {
                    egui::Panel::left("animations tree")
                        .exact_size(TREE_WIDTH)
                        .resizable(false)
                        .show_inside(ui, |ui| {
                            if self.tree.show(
                                ui,
                                ui.make_persistent_id("animations_tree"),
                                collections,
                            ) {
                                self.dirty = false;
                                validate = true;
                            }
                        });

                    egui::Panel::right("animation editor")
                        .exact_size(300.0)
                        .resizable(false)
                        .show_inside(ui, |ui| {
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

                    egui::CentralPanel::default().show_inside(ui, |ui| {
                        if self.tree.common_settings(ui, &mut self.dirty, collections) {
                            validate = true;
                            if let TreeSelection::Asset(animation) = &self.tree.selected() {
                                UiAction::ReloadShaderCode(Some(animation.id)).enqueue();
                            }
                        }

                        if let TreeSelection::Asset(animation) = &mut self.tree.selected() {
                            if let Some(mut error) = self.error.as_deref() {
                                let lines = error.lines().count().max(1);
                                egui::Frame::NONE
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
                                        .context_menu(
                                            |ui| {
                                                if ui.button("🖹 Copy error").clicked() {
                                                    ui.ctx().copy_text(error.to_owned());
                                                    ui.close_kind(UiKind::Menu);
                                                }
                                            },
                                        );
                                    });
                            }

                            if animation.data.change_shader_code_ui(ui) {
                                validate = true;
                                self.dirty = true;
                            }
                        }
                    });
                });
            },
        );

        if validate {
            self.validate(collections);
        }
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn show_preview_window(
        &mut self,
        ctx: &Context,
        collections: &mut Collections,
        persistent_state: &mut PersistentState,
        sound_data: &mut SoundData,
    ) {
        ctx.show_viewport_immediate(
            ViewportId(Id::new("animation preview window")),
            default_viewport_builder()
                .with_inner_size(Vec2::new(630.0, 440.0))
                .with_min_inner_size(Vec2::new(630.0, 440.0)),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.preview = false;
                    }
                });

                gled_window_frame(ctx, "Animation Preview", |ui| {
                    if let Some(effect) = &mut self.effect {
                        egui::Panel::right("animation preview right side")
                            .exact_size(300.0)
                            .resizable(false)
                            .show_inside(ui, |ui| {
                                ScrollArea::vertical()
                                    .scroll_bar_visibility(AlwaysVisible)
                                    .max_height(ui.available_height())
                                    .show(ui, |ui| {
                                        effect.config_ui(
                                            ui,
                                            false,
                                            None,
                                            1.0,
                                            collections,
                                            sound_data,
                                        )
                                    });
                            });
                    }

                    egui::CentralPanel::default().show_inside(ui, |ui| {
                        egui::Frame::NONE
                            .inner_margin(Margin::from(6.0))
                            .stroke(Stroke::new(1.0, Color32::DARK_GRAY))
                            .show(ui, |ui| {
                                ui.label("Preview Palette");
                                ui.vertical_centered_justified(|ui| {
                                    if persistent_state
                                        .preview_palette_mut()
                                        .collections_change_button(ui, collections)
                                    {
                                        persistent_state.save();
                                    }
                                });
                            });

                        ui.add_space(4.0);

                        if let Some(effect) = &mut self.effect {
                            egui::Frame::default()
                                .outer_margin(Margin::same(4))
                                .show(ui, |ui| {
                                    ui.add_sized(
                                        Vec2::splat(300.0),
                                        EffectWidget {
                                            show_group: false,
                                            selectable: None,
                                            effect,
                                            svg: None,
                                            groups: None,
                                            groups_show_index: false,
                                            beat_progression: None,
                                            collections,
                                            sound_data,
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
                });
            },
        );
    }
}
