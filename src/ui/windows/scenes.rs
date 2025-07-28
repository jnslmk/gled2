use crate::{
    app::{persistant_state::PersistantState, timing::Timing},
    pipeline::renderer_callback::RendererCallback,
    storage::{
        asset::{
            Asset,
            animation::Animation,
            scene::{Scene, effect::Effect, effect_state::EffectState},
        },
        asset_id::AssetId,
    },
    ui::{
        ChangeButton,
        asset_tree::{AssetTree, TREE_WIDTH, TreeSelection},
        effect::widget::EffectWidget,
        viewport_builder::default_viewport_builder,
    },
    wgpu_render_state,
};
use egui::{
    Button, Color32, Context, Id, Margin, Stroke, Vec2, ViewportId,
    scroll_area::ScrollBarVisibility,
};
use wgpu::CommandEncoderDescriptor;

#[derive(Default)]
pub struct ScenesWindow {
    open: bool,
    dirty: bool,
    tree: AssetTree<Scene>,
    effect_states: Vec<EffectState>,
    selected_effect: usize,
}

impl ScenesWindow {
    pub fn update(&mut self, ctx: &Context, timing: &Timing) {
        if !self.open {
            return;
        }

        ctx.show_viewport_immediate(
            ViewportId(Id::new("scenes window")),
            default_viewport_builder()
                .with_title("Gled: Scenes")
                .with_inner_size(Vec2::new(810.0, 500.0))
                .with_min_inner_size(Vec2::new(810.0, 500.0)),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.open = false;
                    }
                });

                egui::SidePanel::left("scenes tree")
                    .exact_width(TREE_WIDTH)
                    .resizable(false)
                    .show(ctx, |ui| {
                        if self.tree.show(ui, ui.make_persistent_id("scenes_tree")) {
                            self.dirty = false;
                            self.selected_effect = 0;
                            self.effect_states.clear();
                            if let TreeSelection::Asset(scene) = self.tree.selected() {
                                scene.data.init_states(&mut self.effect_states);
                            }
                        }
                    });

                egui::SidePanel::right("scene editor")
                    .exact_width(300.0)
                    .resizable(false)
                    .show(ctx, |ui| {
                        let TreeSelection::Asset(scene) = self.tree.selected() else {
                            return;
                        };

                        if let (Some(effect), Some(state)) = (
                            scene.data.effect(self.selected_effect),
                            self.effect_states.get_mut(self.selected_effect),
                        ) {
                            self.dirty |= effect.config_ui(state, ui, true);
                        }

                        ui.separator();

                        ui.vertical_centered_justified(|ui| {
                            if ui
                                .add(Button::new("+ Add Effect").fill(Color32::DARK_GREEN))
                                .clicked()
                            {
                                self.selected_effect = scene
                                    .data
                                    .add_effect(&mut self.effect_states, Effect::default());
                                self.dirty = true;
                            }
                        });
                        ui.vertical_centered_justified(|ui| {
                            if ui
                                .add(Button::new("🗐 Duplicate Effect").fill(Color32::DARK_BLUE))
                                .clicked()
                            {
                                if let Some(effect) =
                                    scene.data.effect(self.selected_effect).cloned()
                                {
                                    self.selected_effect =
                                        scene.data.add_effect(&mut self.effect_states, effect);
                                    self.dirty = true;
                                }
                            }
                        });
                        ui.vertical_centered_justified(|ui| {
                            if ui
                                .add(Button::new("🗑 Remove Effect").fill(Color32::DARK_RED))
                                .clicked()
                            {
                                scene
                                    .data
                                    .remove_effect(&mut self.effect_states, self.selected_effect);
                                self.selected_effect = self.selected_effect.saturating_sub(1);
                                self.dirty = true;
                            }
                        });
                    });

                egui::CentralPanel::default().show(ctx, |ui| {
                    let asset_changed: bool = self.tree.common_settings(ui, &mut self.dirty);

                    ui.add_space(4.0);

                    if let TreeSelection::Asset(scene) = &mut self.tree.selected() {
                        if asset_changed {
                            crate::ui::action::UiAction::InitGPU.enqueue();
                            if let (Some(effect), Some(state)) = (
                                scene.data.effect(self.selected_effect),
                                self.effect_states.get_mut(self.selected_effect),
                            ) {
                                state.update(effect);
                            }
                        }

                        egui::Frame::NONE
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

                        {
                            for effect_state in self.effect_states.iter_mut() {
                                effect_state.beat_progression = timing.beat_progression();
                                effect_state.beats_per_minute = timing.beats_per_minute;
                                effect_state.framerate = timing.framerate().unwrap_or_default();
                            }

                            let wgpu_render_state = wgpu_render_state();
                            let device = wgpu_render_state.device;
                            let queue = &wgpu_render_state.queue;
                            scene.data.prepare(
                                &mut self.effect_states,
                                queue,
                                PersistantState::get().preview_palette.and_then(Asset::get),
                                &Default::default(),
                                1.0,
                            );
                            let mut encoder =
                                device.create_command_encoder(&CommandEncoderDescriptor {
                                    label: Some("Render animations for scene editor"),
                                });
                            scene
                                .data
                                .render(&mut self.effect_states, &mut encoder, false);
                            RendererCallback::add(encoder.finish());
                        }

                        egui::ScrollArea::vertical()
                            .id_salt("effects_scroll")
                            .auto_shrink([false, false])
                            .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
                            .show(ui, |ui| {
                                ui.set_max_width(ui.available_width() - 30.0);
                                ui.horizontal_wrapped(|ui| {
                                    if let TreeSelection::Asset(scene) = self.tree.selected() {
                                        for (index, (effect, effect_state)) in scene
                                            .data
                                            .effects()
                                            .iter()
                                            .zip(self.effect_states.iter_mut())
                                            .enumerate()
                                        {
                                            ui.add_sized(
                                                Vec2::splat(100.0),
                                                EffectWidget {
                                                    show_group: true,
                                                    selectable: Some((
                                                        &mut self.selected_effect,
                                                        index,
                                                    )),
                                                    effect,
                                                    effect_state,
                                                },
                                            );
                                        }
                                    }
                                });
                            });
                    }
                });
            },
        );
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn reload_shader_code(&mut self, animation: AssetId<Animation>) {
        let TreeSelection::Asset(scene) = self.tree.selected() else {
            return;
        };

        scene
            .data
            .reload_shader_code(&mut self.effect_states, animation);
    }
}
