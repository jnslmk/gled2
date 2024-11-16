use crate::{
    app::Timing,
    effect::{Effect, EffectState},
    group::Groups,
    storage::{Asset, AssetId, Palette, Scene},
    ui::{
        asset_tree::{AssetTree, TreeSelection},
        ChangeButton,
    },
    wgpu_render_state,
};
use egui::{
    epaint::{CircleShape, RectShape},
    pos2,
    scroll_area::ScrollBarVisibility,
    Button, Color32, Context, Label, Margin, Rect, Response, Rounding, Sense, Shape, Ui, Vec2,
    Widget,
};
use std::{
    iter::once,
    time::{SystemTime, UNIX_EPOCH},
};
use wgpu::CommandEncoderDescriptor;

#[derive(Default)]
pub struct ScenesWindow {
    open: bool,
    dirty: bool,
    tree: AssetTree<Scene>,
    palette: Option<AssetId<Palette>>,
    previous_selected_id: Option<AssetId<Scene>>,
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
            .min_width(755.0)
            .resizable(true)
            .default_pos(ctx.available_rect().center())
            .open(&mut self.open)
            .show(ctx, |ui| {
                egui::SidePanel::left("scenes tree")
                    .exact_width(200.0)
                    .resizable(false)
                    .show_inside(ui, |ui| {
                        self.tree.show(ui, ui.make_persistent_id("scenes_tree"));
                    });

                if self.previous_selected_id != self.tree.selected_id() {
                    self.previous_selected_id = self.tree.selected_id();
                    self.selected_effect = 0;
                    self.effect_states.clear();
                    if let TreeSelection::Asset(scene) = self.tree.selected() {
                        scene.data.init_states(&mut self.effect_states);
                    }
                }

                egui::SidePanel::right("scene editor")
                    .exact_width(300.0)
                    .resizable(false)
                    .show_inside(ui, |ui| {
                        let asset_changed = self.tree.common_settings(ui, &mut self.dirty);

                        let TreeSelection::Asset(scene) = self.tree.selected() else {
                            return;
                        };

                        if asset_changed {
                            crate::ui::action::Action::InitGPU.enqueue();
                            if let (Some(effect), Some(state)) = (
                                scene.data.effect(self.selected_effect),
                                self.effect_states.get_mut(self.selected_effect),
                            ) {
                                state.update(effect);
                            }
                        }

                        if let (Some(effect), Some(state)) = (
                            scene.data.effect(self.selected_effect),
                            self.effect_states.get_mut(self.selected_effect),
                        ) {
                            self.dirty |= effect.config_ui(state, ui);
                        }

                        ui.separator();

                        ui.vertical_centered_justified(|ui| {
                            if ui
                                .add(Button::new("+ Add Effect").fill(Color32::DARK_BLUE))
                                .clicked()
                            {
                                self.selected_effect = scene
                                    .data
                                    .add_effect(&mut self.effect_states, Effect::default());
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
                            }
                        });
                    });

                egui::CentralPanel::default().show_inside(ui, |ui| {
                    if let TreeSelection::Asset(scene) = &mut self.tree.selected() {
                        ui.vertical_centered_justified(|ui| {
                            self.palette.change_button(ui);
                        });

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
                                self.palette.and_then(Asset::get),
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
                                                    selected_effect: &mut self.selected_effect,
                                                    index,
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

struct EffectWidget<'a> {
    selected_effect: &'a mut usize,
    index: usize,
    effect: &'a Effect,
    effect_state: &'a EffectState,
}

impl<'a> Widget for EffectWidget<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let response = egui::Frame::none()
            .fill(if *self.selected_effect == self.index {
                Color32::GOLD.linear_multiply(
                    ((SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("Could not get time")
                        .subsec_millis()
                        / 100) as f32
                        / 5.0
                        - 1.0)
                        .abs(),
                )
            } else {
                Color32::TRANSPARENT
            })
            .inner_margin(Margin::from(10.0))
            .rounding(Rounding::from(4.0))
            .show(ui, |ui| {
                let rect = ui.available_rect_before_wrap();
                ui.allocate_rect(rect, Sense::hover());
                ui.painter().add(Shape::Rect(RectShape::filled(
                    rect,
                    Rounding::default(),
                    Color32::BLACK,
                )));
                ui.painter().add(Shape::Rect(RectShape {
                    rect,
                    rounding: Rounding::default(),
                    blur_width: 0.0,
                    fill_texture_id: self.effect_state.texture_id(),
                    uv: Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    fill: Color32::WHITE,
                    stroke: Default::default(),
                }));
                ui.painter().add(Shape::Circle(CircleShape::filled(
                    rect.left_top() + Vec2::splat(10.0),
                    5.5,
                    if self.effect.use_secondary_group {
                        Color32::GOLD
                    } else {
                        Color32::GREEN
                    },
                )));
                ui.put(
                    Rect::from_min_size(rect.left_top() + Vec2::new(0.0, 0.5), Vec2::splat(20.0)),
                    Label::new(
                        egui::RichText::new(if self.effect.use_secondary_group {
                            "S"
                        } else {
                            "P"
                        })
                        .color(Color32::BLACK),
                    ),
                );
            })
            .response;

        let res = ui.put(response.rect, Button::new("").fill(Color32::TRANSPARENT));
        if res.clicked() {
            *self.selected_effect = self.index;
        }

        response
    }
}
