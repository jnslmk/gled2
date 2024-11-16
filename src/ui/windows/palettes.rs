use crate::{
    storage::{Asset, Palette},
    ui::asset_tree::{AssetTree, TreeSelection},
};
use egui::{Button, Context, Margin, Ui};
use std::sync::Arc;

#[derive(Default)]
pub struct PalettesWindow {
    open: bool,
    dirty: bool,
    tree: AssetTree<Palette>,
}

impl PalettesWindow {
    pub fn update(&mut self, ctx: &Context) {
        if !self.open {
            self.dirty = false;
            return;
        }

        egui::Window::new("Palettes")
            .collapsible(false)
            .min_width(500.0)
            .resizable(true)
            .default_pos(ctx.available_rect().center())
            .open(&mut self.open)
            .show(ctx, |ui| {
                egui::SidePanel::left("palettes tree")
                    .exact_width(300.0)
                    .resizable(false)
                    .show_inside(ui, |ui| {
                        self.tree.show(ui, ui.make_persistent_id("palettes_tree"));
                    });

                egui::Frame::default().outer_margin(Margin::same(4.0)).show(
                    ui,
                    |ui| match &mut self.tree.selected() {
                        TreeSelection::None => {
                            ui.label("Please select an item from the tree");
                        }
                        TreeSelection::Asset(palette) => {
                            palette_editor(ui, palette, &mut self.dirty);
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

fn palette_editor(ui: &mut Ui, palette: &mut Asset<Palette>, dirty: &mut bool) {
    ui.label("Name:");
    let mut name = palette.name().to_string();
    let res = ui.text_edit_singleline(&mut name);
    if res.changed() {
        *dirty = true;
        palette.path.pop();
        palette.path.push(name);
    }

    ui.label("Primary color:");
    let res = ui.color_edit_button_rgb(palette.data.primary.rgb_mut());
    *dirty |= res.changed();

    ui.label("Secondary color:");
    let res = ui.color_edit_button_rgb(palette.data.secondary.rgb_mut());
    *dirty |= res.changed();

    ui.label("Gradient colors:");
    ui.scope(|ui| {
        ui.horizontal_wrapped(|ui| {
            palette.data.gradient.iter_mut().for_each(|color| {
                let res = ui.color_edit_button_rgb(color.rgb_mut());
                *dirty |= res.changed();
            });
        });
    });

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui
            .add_enabled(*dirty, Button::new("Save"))
            .on_hover_ui(|ui| {
                ui.label("Save the palette to disk");
            })
            .clicked()
        {
            *dirty = false;
            palette.clone().save();
        }
        if ui
            .add_enabled(*dirty, Button::new("Reset"))
            .on_hover_ui(|ui| {
                ui.label("Reset to state on disk");
            })
            .clicked()
        {
            *dirty = false;
            *palette = Arc::unwrap_or_clone(Asset::get(palette.id).unwrap_or_default());
        }
    });
}
