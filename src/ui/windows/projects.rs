use crate::{
    storage::{asset::project::Project, collections::Collections},
    ui::{
        action::UiAction,
        asset_tree::{AssetTree, TreeSelection},
        window_common::{default_viewport_builder, gled_window_frame},
    },
};
use egui::{Button, Id, Vec2, ViewportId};
use log::debug;

#[derive(Default)]
pub struct ProjectsWindow {
    open: bool,
    dirty: bool,
    tree: AssetTree<Project>,
}

impl ProjectsWindow {
    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn update(&mut self, ctx: &egui::Context, collections: &mut Collections) {
        if !self.open {
            return;
        }

        ctx.show_viewport_immediate(
            ViewportId(Id::new("projects window")),
            default_viewport_builder()
                .with_inner_size(Vec2::new(550.0, 500.0))
                .with_min_inner_size(Vec2::new(550.0, 500.0)),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.open = false;
                    }
                });

                gled_window_frame(ctx, "Projects", |ui| {
                    egui::SidePanel::left("projects tree")
                        .exact_width(200.0)
                        .resizable(false)
                        .show_inside(ui, |ui| {
                            if self.tree.show(
                                ui,
                                ui.make_persistent_id("projects_tree"),
                                collections,
                            ) {
                                self.dirty = false;
                            }
                        });

                    egui::CentralPanel::default().show_inside(ui, |ui| {
                        self.tree.common_settings(ui, &mut self.dirty, collections);
                        if let TreeSelection::Asset(project) = self.tree.selected() {
                            ui.label(format!(
                                "Scenes in Grid: {}",
                                project.data.scenes_instances_grid_len()
                            ));
                            ui.vertical_centered_justified(|ui| {
                                if ui.button("Load Project").clicked() {
                                    UiAction::SetProject(project.id).enqueue();
                                    self.open = false;
                                }
                            });
                            ui.vertical_centered_justified(|ui| {
                                if ui
                                    .add_enabled(
                                        project.data.svg.is_some(),
                                        Button::new("🖻 Export SVG file"),
                                    )
                                    .clicked()
                                {  
                                    let svg = project.data.svg.as_ref().cloned();
                                    std::thread::Builder::new()
                                        .name("gled:ui:save_svg".to_string())
                                        .spawn(move || {
                                            #[cfg(feature = "profiling")]
                                            profiling::register_thread!("save_svg_file");

                                            if let (Some(svg), Some(path)) = (
                                                svg,
                                                rfd::FileDialog::new()
                                                    .set_title("Save SVG file")
                                                    .add_filter("svg", &["svg"])
                                                    .save_file(),
                                            ) {
                                                match svg.save(&path) {
                                                    Ok(_) => {
                                                        debug!("Saved svg file \"{}\"", path.display());
                                                    }
                                                    Err(err) => {
                                                        UiAction::Error(format!(
                                                            "Could not save svg file \"{}\": {err:?}",
                                                            path.display()
                                                        ))
                                                        .enqueue();
                                                    }
                                                };
                                            }
                                        })
                                        .ok();
                                }
                            });
                        }
                    });
                });
            },
        );
    }

    pub fn open(&mut self) {
        self.open = true;
    }
}
