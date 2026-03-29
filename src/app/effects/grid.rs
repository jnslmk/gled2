use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use super::App;
use crate::{midi::akai_apc40_mk2::{GRID_HEIGHT, GRID_WIDTH}, storage::asset::scene::grid::GridLocation, ui::{ContextMenuAction, ContextMenuBuilder, action::UiAction, scene_instance::{dnd::{dnd_drag_source, dnd_drop_zone}, widget::{EmptyGridSpot, SceneInstanceWidget}}}};
use egui::{
    Color32, Frame, Id, KeyboardShortcut, Modifiers, TextureHandle, Ui, UiBuilder, Vec2,
    scroll_area::ScrollBarVisibility::AlwaysVisible,
};
use egui::{DragAndDrop, Label, LayerId, Order, Response, Sense, Widget};
use egui_phosphor_icons::icons;
use emath::{Rect, vec2};
use epaint::{Stroke, StrokeKind};

impl App {
    pub fn effects_grid(&mut self, ui: &mut Ui, svg: Option<TextureHandle>) {
        egui::ScrollArea::both()
            .id_salt("grid scroll")
            .auto_shrink([false, false])
            .scroll_bar_visibility(AlwaysVisible)
            .show(ui, |ui| {
                let Some(project) = self.project.as_mut() else {
                    return;
                };
                let effects_size = self.persistant_state.effects_size();
                ui.set_width(GRID_WIDTH as f32 * (effects_size + 20.) + 20.);
                ui.set_height(GRID_HEIGHT as f32 * (effects_size + 20.) + 20.);

                let to_global = ui
                    .ctx()
                    .layer_transform_to_global(ui.layer_id())
                    .unwrap_or_default();
                let groups = project.groups.clone();

                let grid = &mut project.scenes_instances_grid;
                let start_pos = ui.cursor().min;

                let mut dropped = None;
                let mut clicked_selection = None;
                for row in 0..GRID_HEIGHT {
                    if row == GRID_HEIGHT - 1 {
                        let quick_scene_rect = Rect::from_min_size(
                            start_pos + vec2(20., 20. + row as f32 * (effects_size + 20.)),
                            vec2((effects_size + 20.) * GRID_WIDTH as f32 - 20., effects_size),
                        )
                        .expand(10.);
                        ui.painter().rect_filled(
                            quick_scene_rect,
                            5.,
                            Color32::GOLD.blend(Color32::from_black_alpha(200)),
                        );
                    }

                    for col in 0..GRID_WIDTH {
                        let location = GridLocation { col, row };

                        let rect = to_global.mul_rect(Rect::from_min_size(
                            start_pos
                                + vec2(20., 20.)
                                + vec2(
                                    col as f32 * (effects_size + 20.),
                                    row as f32 * (effects_size + 20.),
                                ),
                            vec2(effects_size, effects_size),
                        ));
                        let (_, dropped_payload) = ui
                            .scope_builder(UiBuilder::new().max_rect(rect), |ui| {
                                dnd_drop_zone::<GridLocation, ()>(
                                    ui,
                                    Frame::default().corner_radius(2.),
                                    |ui| {
                                        let scene = grid.get_mut(&location);
                                        match scene {
                                            Some(scene_instance) => {
                                                let item_id = Id::new((
                                                    "Draggable Scene Widget",
                                                    scene_instance.id,
                                                ));
                                                let mut widget_response =
                                                    dnd_drag_source(ui, item_id, location, |ui| {
                                                        ui.add(SceneInstanceWidget {
                                                            scene_instance,
                                                            svg: svg.clone(),
                                                            size: Vec2::splat(effects_size),
                                                            groups: &groups,
                                                            timing: &self.timing,
                                                            collections: &self.collections,
                                                            effects_size,
                                                            sound_data: &self.sound_data,
                                                        })
                                                    });

                                                // setup context actions
                                                ContextMenuBuilder::default()
                                                    .add_action(ContextMenuAction {
                                                        description: "Duplicate Scene".to_string(),
                                                        keyboard_shortcut: KeyboardShortcut::new(
                                                            Modifiers::COMMAND,
                                                            egui::Key::D,
                                                        ),
                                                        ctx_action: |location| {
                                                            UiAction::CloneSceneInstance(location)
                                                                .enqueue();
                                                        },
                                                        key_action: || {
                                                            UiAction::CloneSelectedSceneInstance
                                                                .enqueue();
                                                        },
                                                    })
                                                    .add_action(ContextMenuAction {
                                                        description: "Delete Scene".to_string(),
                                                        keyboard_shortcut: KeyboardShortcut::new(
                                                            Modifiers::default(),
                                                            egui::Key::Delete,
                                                        ),
                                                        ctx_action: |location| {
                                                            UiAction::DeleteSceneInstance {
                                                                location,
                                                            }
                                                            .enqueue();
                                                        },
                                                        key_action: || {
                                                            UiAction::DeleteSelectedSceneInstance
                                                                .enqueue();
                                                        },
                                                    })
                                                    .show(&mut widget_response, location);

                                                // also handle backspace as delete action for MacOS
                                                if ui.ctx().input_mut(|i| {
                                                    i.consume_key(
                                                        Modifiers::default(),
                                                        egui::Key::Backspace,
                                                    )
                                                }) {
                                                    UiAction::DeleteSelectedSceneInstance.enqueue();
                                                }

                                                if ui.ctx().is_being_dragged(item_id) {
                                                    ui.painter().rect_filled(
                                                        widget_response.rect,
                                                        5.0,
                                                        Color32::from_gray(100),
                                                    );
                                                } else if self.selected_scene_instance == location {
                                                    ui.painter().rect_stroke(
                                                        widget_response.rect,
                                                        5.0,
                                                        Stroke::new(2., Color32::from_gray(200)),
                                                        StrokeKind::Outside,
                                                    );
                                                } else if widget_response.hovered() {
                                                    ui.painter().rect_stroke(
                                                        widget_response.rect,
                                                        5.0,
                                                        Stroke::new(1., Color32::from_gray(160)),
                                                        StrokeKind::Outside,
                                                    );
                                                }
                                                if widget_response.clicked() {
                                                    clicked_selection = Some(location);
                                                }
                                            }
                                            None => {
                                                ui.add(EmptyGridSpot {
                                                    location,
                                                    collections: &mut self.collections,
                                                    effects_size,
                                                });
                                            }
                                        };
                                    },
                                )
                            })
                            .inner; // dnd_drop_zone

                        if let Some(payload) = dropped_payload {
                            dropped = Some((payload, row, col));
                        }
                    }
                }

                if let Some((location, row, col)) = dropped {
                    // The user dropped onto this cell
                    let from = *location;
                    let to = GridLocation { row, col };

                    // Ownership of the scene instances must temporarily be taken to swap
                    let to_item = grid.remove(&to);
                    let from_item = { grid.remove(&from) };
                    if let Some(from_item) = from_item {
                        grid.insert(to, from_item);
                    }
                    // reinsert the to item to the from location
                    if let Some(to_item) = to_item {
                        grid.insert(from, to_item);
                    }
                    UiAction::SelectSceneByLocation(to).enqueue();
                }

                if let Some(location) = clicked_selection {
                    self.set_selected_scene_instance(location);
                }

                static SHOW_TRASH_ONE_MORE_FRAME: AtomicBool = AtomicBool::new(false);
                let is_dragging = DragAndDrop::has_payload_of_type::<GridLocation>(ui.ctx());
                if is_dragging || SHOW_TRASH_ONE_MORE_FRAME.load(Relaxed) {
                    SHOW_TRASH_ONE_MORE_FRAME.store(is_dragging, Relaxed);
                    let id = Id::new("Trash");
                    let layer_id = LayerId::new(Order::Foreground, id);
                    ui.scope_builder(UiBuilder::new().layer_id(layer_id), |ui| {
                        ui.set_clip_rect(ui.ctx().content_rect());
                        let rect = Rect::from_min_size(
                            ui.ctx().content_rect().left_bottom()
                                + vec2(10.0, -(effects_size + 10.0)),
                            Vec2::splat(effects_size),
                        );
                        ui.place(rect, TrashWidget { effects_size });
                    });
                }
            });
    }
}

pub struct TrashWidget {
    effects_size: f32,
}

impl Widget for TrashWidget {
    fn ui(self, ui: &mut Ui) -> Response {
        let (response, dropped) =
            dnd_drop_zone::<GridLocation, ()>(ui, Frame::default().corner_radius(2.), |ui| {
                let rect = Rect::from_min_size(ui.cursor().min, Vec2::splat(self.effects_size));
                ui.painter()
                    .rect_filled(rect, 5.0, Color32::from_rgb(150, 0, 0));
                ui.place(
                    rect,
                    Label::new(icons::TRASH.regular().size(40.0).color(Color32::WHITE)),
                );

                ui.allocate_rect(rect, Sense::hover());
            });
        if let Some(dropped) = dropped {
            UiAction::DeleteSceneInstance { location: *dropped }.enqueue();
        }

        response.response
    }
}
