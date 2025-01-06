use crate::{
    storage::asset::project::{scene_instance_path::SceneInstancePath, Project},
    ui::{
        action::UiAction,
        asset_tree::{AssetTree, TreeSelection},
        viewport_builder::default_viewport_builder,
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
    pub fn update(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }

        ctx.show_viewport_immediate(
            ViewportId(Id::new("projects window")),
            default_viewport_builder()
                .with_title("Gled: Projects")
                .with_inner_size(Vec2::new(500.0, 500.0))
                .with_min_inner_size(Vec2::new(500.0, 500.0)),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.open = false;
                    }
                });

                egui::SidePanel::left("projects tree")
                    .exact_width(200.0)
                    .resizable(false)
                    .show(ctx, |ui| {
                        if self.tree.show(ui, ui.make_persistent_id("projects_tree")) {
                            self.dirty = false;
                        }
                    });

                egui::CentralPanel::default().show(ctx, |ui| {
                    self.tree.common_settings(ui, &mut self.dirty);
                    if let TreeSelection::Asset(project) = self.tree.selected() {
                        ui.label(format!(
                            "Scenes in Deck A: {}",
                            project
                                .data
                                .scene_instances(SceneInstancePath::DECK_A)
                                .count()
                        ));
                        ui.label(format!(
                            "Scenes in Deck B: {}",
                            project
                                .data
                                .scene_instances(SceneInstancePath::DECK_B)
                                .count()
                        ));
                        ui.label(format!(
                            "Scenes in Common Deck: {}",
                            project
                                .data
                                .scene_instances(SceneInstancePath::DECK_C)
                                .count()
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
                                std::thread::spawn(move || {
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
                                });
                            }
                        });
                    }
                });
            },
        );
    }

    pub fn open(&mut self) {
        self.open = true;
    }
}
