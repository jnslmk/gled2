use super::App;
use crate::storage::asset::scene::grid::GridLocation;
use crate::ui::scene_instance::dnd::{dnd_drag_source, dnd_drop_zone};
use crate::ui::scene_instance::widget::SceneInstanceWidget;
use crate::ui::scene_instance::widget::{EmptyGridSpot, SCENE_WIDGET_SIZE};
use crate::app::PersistantState;
use egui::{
    scroll_area::ScrollBarVisibility::AlwaysVisible, Color32, Frame, Id, TextureHandle, Ui, UiBuilder,
    Vec2,
};
use emath::{vec2, Rect};
use epaint::{Stroke, StrokeKind};
use crate::midi::akai_apc40_mk2::{GRID_HEIGHT, GRID_WIDTH};

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
                let to_global = ui.ctx().layer_transform_to_global(ui.layer_id()).unwrap_or_default();
                let effects_size = PersistantState::effects_size();
                let groups = project.groups.clone();

                let mut grid = &mut project.scenes_instances_grid;
                let start_pos = ui.cursor().min + vec2(20., 20.);

                    for row in 0..GRID_HEIGHT {
                        for col in 0..GRID_WIDTH {
                            let location = GridLocation { col, row };

                            let tile_length = SCENE_WIDGET_SIZE + 28.;
                            let rect = to_global.mul_rect
                            (Rect::from_min_size(
                                start_pos + vec2(col as f32 * (tile_length + 20.), row as f32 * (tile_length + 20.)),
                                vec2(tile_length, tile_length )));
                            let (_, dropped_payload) = ui
                                .scope_builder(UiBuilder::new().max_rect(rect),|ui| {
                                    dnd_drop_zone::<GridLocation, ()>(ui, Frame::default().corner_radius(2.), |ui| {
                                        let scene = grid.get_mut(&location);
                                        match scene {
                                            Some(scene_instance) => {
                                                let item_id = Id::new((
                                                    "Draggable Scene Widget",
                                                    scene_instance.id,
                                                ));
                                                let widget_response =
                                                    dnd_drag_source(ui, item_id, location, |ui| {
                                                        ui.add(SceneInstanceWidget {
                                                            selected_scene_instance: &mut self
                                                                .selected_scene_instance,
                                                            scene_instance,
                                                            svg: svg.clone(),
                                                            size: Vec2::splat(effects_size),
                                                            groups: &groups,
                                                            timing: &self.timing,
                                                        })
                                                    });


                                                if self.selected_scene_instance.id
                                                    == scene_instance.id
                                                {
                                                    ui.painter().rect_stroke(
                                                        widget_response.rect,
                                                        5.0,
                                                        Stroke::new(2., Color32::WHITE),
                                                        StrokeKind::Outside,
                                                    );
                                                }
                                                else {
                                                    if widget_response.hovered() {
                                                        ui.painter().rect_stroke(
                                                            widget_response.rect,
                                                            5.0,
                                                            Stroke::new(2., Color32::from_gray(160)),
                                                            StrokeKind::Outside,
                                                        );
                                                    }
                                                }
                                                if widget_response.clicked(){
                                                    self.selected_scene_instance.id = scene_instance.id;
                                                }
                                            } // Some(scene) =>
                                            None => {
                                                let init_gpu = false;
                                                ui.add(EmptyGridSpot {
                                                    selected_scene_instance: &mut self
                                                        .selected_scene_instance,
                                                    grid: &mut grid,
                                                    location,
                                                    init_gpu: &init_gpu,
                                                });
                                            }
                                        };
                                    })
                                })
                                .inner; // dnd_drop_zone
                            if let Some(dragged_payload) = dropped_payload {
                                // The user dropped onto this cell
                                let from = GridLocation {
                                    row: dragged_payload.row,
                                    col: dragged_payload.col,
                                };
                                let to = GridLocation { row, col };

                                // Ownership of the scene instances must temporarily be taken to swap
                                let to_item = grid.remove(&to);
                                let from_item = { grid.remove(&from) };
                                if let Some(from_item) = from_item {
                                    self.selected_scene_instance.id = from_item.id;
                                    grid.insert(to, from_item);
                                }
                                // reinsert the to item to the from location
                                if let Some(to_item) = to_item {
                                    grid.insert(from, to_item);
                                }
                            }
                        }
                    } // grid
            });
    }
}
