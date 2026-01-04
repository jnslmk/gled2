use super::App;
use crate::storage::asset::scene::grid::GridLocation;
use crate::ui::scene_instance::widget::EmptyGridSpot;
use crate::ui::scene_instance::widget::SceneInstanceWidget;
use crate::{
    app::PersistantState,
    storage::asset::project::DeckPath,
};
use egui::{scroll_area::ScrollBarVisibility::AlwaysVisible, Frame, Grid, Id, TextureHandle, Ui, Vec2};

pub const WIDTH: usize = 6;
pub const HEIGHT: usize = 4;

impl App {
    pub fn effects_grid(&mut self, ui: &mut Ui, svg: Option<TextureHandle>) {
        egui::ScrollArea::vertical()
            .id_salt("grid scroll")
            .auto_shrink([false, false])
            .scroll_bar_visibility(AlwaysVisible)
            .show(ui, |ui| {
                ui.set_max_width(ui.available_width() - 30.0);
                ui.horizontal_wrapped(|ui| {
                    self.widgets(ui, svg, DeckPath::Grid);
                });
            });
    }

    fn widgets(&mut self, ui: &mut Ui, svg: Option<TextureHandle>, deck_path: DeckPath) {
        let Some(project) = self.project.as_mut() else {
            return;
        };
        let effects_size = PersistantState::effects_size();
        let groups = project.groups.clone();

        let size = Vec2::splat(effects_size);

        let mut grid = match deck_path {
            DeckPath::Grid => &mut project.scenes_instances_grid,
            DeckPath::Quick => &mut project.scenes_instances_quick,
        };



            //ScrollArea::both().show(ui, |ui| {
            Grid::new(deck_path).show(ui, |ui| {
                for row in 0..HEIGHT {
                    for col in 0..WIDTH {
                        let location = GridLocation { col, row };

                        let frame = Frame::default().outer_margin(3.0);
                        let (_, dropped_payload) = ui.dnd_drop_zone::<GridLocation, ()>(frame, |ui| {
                            let scene = grid.get_mut(&location);
                                match scene {
                                    Some(scene_instance) => {
                                        let item_id = Id::new(("Draggable Scene Widget", scene_instance.id));
                                        ui
                                            .dnd_drag_source(item_id, location, |ui| {
                                                ui.add(SceneInstanceWidget {
                                                    selected_scene_instance: &mut self.selected_scene_instance,
                                                    deck_path,
                                                    scene_instance,
                                                    svg: svg.clone(),
                                                    size,
                                                    groups: &groups,
                                                    timing: &self.timing,
                                                })
                                            })
                                            .response;
                                    } // Some(scene) =>
                                    None => {
                                        let init_gpu = false;
                                        ui.add(EmptyGridSpot {
                                            selected_scene_instance: &mut self.selected_scene_instance,
                                            grid: &mut grid,
                                            location,
                                            init_gpu: &init_gpu,
                                        });
                                    }
                                };
                            }); // dnd_drop_zone
                        if let Some(dragged_payload) = dropped_payload {
                            // The user dropped onto this cell
                            let from = GridLocation{row: dragged_payload.row, col: dragged_payload.col};
                            let to = GridLocation{row, col};

                            // Ownership of the scene instances must temporarily be taken to swap
                            let to_item = grid.remove(&to);
                            // unwrap is safe because we cannot move from an empty tile
                            let from_item = {
                                let a = grid.remove(&from);
                                a.unwrap()
                            };
                            grid.insert(to, from_item);
                            // reinsert the to item to the from location
                            if let Some(to_item) = to_item {
                                grid.insert(from, to_item);
                            }
                        }
                    }
                    ui.end_row();
                } // grid
            });
            //});
        }

}
