use crate::{
    app::{PersistantState, Timing},
    effect::{Effect, EffectState},
    group::Groups,
    storage::{Asset, Scene},
    ui::{
        asset_tree::{AssetTree, TreeSelection, TREE_WIDTH},
        effect::EffectWidget,
        ChangeButton,
    },
    wgpu_render_state,
};
use egui::{scroll_area::ScrollBarVisibility, Button, Color32, Context, Margin, Stroke, Vec2};
use std::iter::once;
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

        egui::Window::new("Scenes")
            .collapsible(false)
            .min_width(810.0)
            .resizable(true)
            .default_pos(ctx.available_rect().center())
            .open(&mut self.open)
            .show(ctx, |ui| {
                egui::SidePanel::left("scenes tree")
                    .exact_width(TREE_WIDTH)
                    .resizable(false)
                    .show_inside(ui, |ui| {
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
                    .show_inside(ui, |ui| {
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

                egui::CentralPanel::default().show_inside(ui, |ui| {
                    let asset_changed: bool = self.tree.common_settings(ui, &mut self.dirty);

                    ui.add_space(4.0);

                    if let TreeSelection::Asset(scene) = &mut self.tree.selected() {
                        if asset_changed {
                            crate::ui::action::Action::InitGPU.enqueue();
                            if let (Some(effect), Some(state)) = (
                                scene.data.effect(self.selected_effect),
                                self.effect_states.get_mut(self.selected_effect),
                            ) {
                                state.update(effect);
                            }
                        }

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
                                &Groups::None,
                                1.0,
                            );
                            let mut encoder =
                                device.create_command_encoder(&CommandEncoderDescriptor {
                                    label: Some("Render animations for scene editor"),
                                });
                            scene
                                .data
                                .render(&mut self.effect_states, &mut encoder, false);
                            queue.submit(once(encoder.finish()));
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
            });
    }

    pub fn open(&mut self) {
        self.open = true;
    }
}
