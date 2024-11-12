use crate::{
    storage::{Asset, Scene},
    ui::asset_tree::{AssetTree, TreeSelection},
};
use egui::{Button, Context, Margin, Ui};
use std::sync::Arc;

#[derive(Default)]
pub struct ScenesWindow {
    open: bool,
    dirty: bool,
    tree: AssetTree<Scene>,
}

impl ScenesWindow {
    pub fn update(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }

        egui::Window::new("Scenes")
            .collapsible(false)
            .min_width(500.0)
            .resizable(true)
            .default_pos(ctx.available_rect().center())
            .open(&mut self.open)
            .show(ctx, |ui| {
                egui::SidePanel::left("scenes tree")
                    .exact_width(300.0)
                    .resizable(false)
                    .show_inside(ui, |ui| {
                        self.tree.show(ui);
                    });

                egui::Frame::default().outer_margin(Margin::same(4.0)).show(
                    ui,
                    |ui| match &mut self.tree.selected() {
                        TreeSelection::None => {
                            ui.label("Please select an item from the tree");
                        }
                        TreeSelection::Asset(palette) => {
                            scene_editor(ui, palette, &mut self.dirty);
                            self.tree.show_delete_button(ui);
                        }
                        TreeSelection::Dir { .. } => {
                            self.tree.show_folder_editor(ui);
                        }
                    },
                );
            });
    }

    pub fn open(&mut self) {
        self.open = true;
    }
}

fn scene_editor(ui: &mut Ui, scene: &mut Asset<Scene>, dirty: &mut bool) {
    ui.label("Name:");
    let mut name = scene.name().to_string();
    let res = ui.text_edit_singleline(&mut name);
    if res.changed() {
        *dirty = true;
        scene.path.pop();
        scene.path.push(name);
    }

    if scene.data.first_effect_mut().config_ui(ui) {
        *dirty = true;
    }

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui
            .add_enabled(*dirty, Button::new("Save"))
            .on_hover_ui(|ui| {
                ui.label("Save the scene to disk");
            })
            .clicked()
        {
            *dirty = false;
            scene.clone().save();
        }
        if ui
            .add_enabled(*dirty, Button::new("Reset"))
            .on_hover_ui(|ui| {
                ui.label("Reset to state on disk");
            })
            .clicked()
        {
            *dirty = false;
            *scene = Arc::unwrap_or_clone(Asset::get(scene.id).unwrap_or_default());
        }
    });
}
