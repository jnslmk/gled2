use super::App;
use crate::storage::asset::project::GridHighlight;
use egui::{DragValue, Slider, Ui};

impl App {
    pub fn effects_header(&mut self, ui: &mut Ui) {
        let Some(project) = self.project.as_mut() else {
            return;
        };

        ui.horizontal(|ui| {
            ui.menu_button("Grid", |ui| {
                ui.label("Grid Size");

                let mut grid_width = project.grid_width();
                let mut grid_height = project.grid_height();

                ui.horizontal(|ui| {
                    ui.label("Width");
                    ui.add(DragValue::new(&mut grid_width).range(2..=64));
                    ui.label("Height");
                    ui.add(DragValue::new(&mut grid_height).range(2..=64));
                });

                if grid_width != project.grid_width() || grid_height != project.grid_height() {
                    project.set_grid_size(grid_width, grid_height);

                    let max_col = project.grid_width() - 1;
                    let max_row = project.grid_height() - 1;
                    self.selected_scene_instance.col =
                        self.selected_scene_instance.col.min(max_col);
                    self.selected_scene_instance.row =
                        self.selected_scene_instance.row.min(max_row);
                }

                ui.separator();
                ui.label("Highlight");
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut project.grid_highlight,
                        GridHighlight::Row,
                        "Last Row",
                    );
                    ui.selectable_value(
                        &mut project.grid_highlight,
                        GridHighlight::Column,
                        "Last Column",
                    );
                    ui.selectable_value(&mut project.grid_highlight, GridHighlight::None, "None");
                });
            });

            ui.menu_button(
                if project.auto_mode_active {
                    "💂Automatic"
                } else {
                    "🔨Manual"
                },
                |ui| {
                    ui.label("Automatic mode");
                    ui.checkbox(&mut project.auto_mode_active, "");
                    ui.label("Seconds/scene");
                    ui.add_enabled(
                        project.auto_mode_active,
                        Slider::new(&mut project.auto_mode_seconds, 1..=240)
                            .custom_formatter(|n, _| format!("{n} s")),
                    );
                    ui.label("Max concurrent scenes");
                    ui.add_enabled(
                        project.auto_mode_active,
                        Slider::new(&mut project.auto_mode_max_scenes, 1..=10),
                    );
                },
            );
        });
    }
}
