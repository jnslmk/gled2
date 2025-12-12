use super::App;
use crate::storage::asset::scene::grid::GridLocation;
use crate::ui::scene_instance::widget::EmptyGridSpot;
use crate::ui::scene_instance::widget::SceneInstanceWidget;
use crate::{
    app::PersistantState,
    storage::asset::project::DeckPath,
};
use egui::{scroll_area::ScrollBarVisibility::AlwaysVisible, Frame, Grid, Id, TextureHandle, Ui, Vec2};

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

    pub fn effects_quick(&mut self, ui: &mut Ui, svg: Option<TextureHandle>) {
        egui::ScrollArea::horizontal()
            .id_salt("quick scroll")
            .auto_shrink([false, false])
            .scroll_bar_visibility(AlwaysVisible)
            .vscroll(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    self.widgets(ui, svg, DeckPath::Quick);
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

        let width = 6;
        let height = 4;
            //ScrollArea::both().show(ui, |ui| {
            Grid::new(deck_path).show(ui, |ui| {
                for row in 0..height {
                    for col in 0..width {
                        let scene = grid.get_mut(&GridLocation { row, col });
                        let frame = Frame::default();
                        let (_, dropped_payload) =
                            ui.dnd_drop_zone::<GridLocation, ()>(frame, |ui| {
                                match scene {
                                    Some(scene_instance) => {
                                        let item_id = Id::new(("Draggable Scene Widget", col, row, deck_path));
                                        let item_location = GridLocation { col, row };
                                        let response = ui
                                            .dnd_drag_source(item_id, item_location, |ui| {
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
                                        ui.add(EmptyGridSpot {});
                                    }
                                };
                            }); // dnd_drop_zone
                        if let Some(dragged_payload) = dropped_payload {
                            // The user dropped onto this cell
                            let from = GridLocation{row: dragged_payload.row, col: dragged_payload.col};
                            let to = GridLocation{row, col};

                            // TODO
                            //grid.swap(from, to);
                        }
                    }
                    ui.end_row();
                } // grid
            });
            //});
        }

}
