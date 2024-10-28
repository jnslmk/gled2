use crate::{
    storage::{Asset, AssetId, AssetTrait, Palette},
    ui::asset_tree::{AssetTree, TreeSelection},
};
use egui::{Context, Label, Layout, Margin, Rect, Shape, Ui, Vec2};
use egui_ltreeview::{node::NodeBuilder, Action, TreeView};
use std::sync::Arc;

#[derive(Default)]
pub struct PalettesWindow {
    pub open: bool,
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
                        self.tree.show(ui);
                    });

                egui::Frame::default().outer_margin(Margin::same(4.0)).show(
                    ui,
                    |ui| match &mut self.tree.selected() {
                        TreeSelection::None => {
                            ui.label("Please select an item from the tree");
                        }
                        TreeSelection::Asset(palette) => {
                            palette_editor(ui, palette, &mut self.dirty);
                        }
                        TreeSelection::Dir { current, new } => {
                            folder_editor(ui, current, new, &mut self.dirty);
                        }
                    },
                );
            });
    }

    pub fn open(&mut self) {
        self.open = true;
    }
}

pub fn folder_editor(ui: &mut Ui, current: &Vec<String>, new: &mut Vec<String>, dirty: &mut bool) {
    ui.label("Name:");
    let mut name = new.last().cloned().unwrap_or_default();
    let res = ui.text_edit_singleline(&mut name);
    if res.changed() {
        *dirty = true;
        new.pop();
        new.push(name);
    }

    if *dirty
        && ui
            .button("Save")
            .on_hover_ui(|ui| {
                ui.label("Save all palettes in this folder with new name to disk");
            })
            .clicked()
    {
        *dirty = false;

        let palettes = Palette::all();
        for palette in palettes {
            if &palette.dir() == current {
                let mut palette = Arc::unwrap_or_clone(palette);
                palette.change_dir(new);
                AssetTrait::save(palette);
            }
        }
    }
}

pub fn palette_editor(ui: &mut Ui, palette: &mut Asset<Palette>, dirty: &mut bool) {
    ui.label("Name:");
    let mut name = palette.name.last().cloned().unwrap_or_default();
    let res = ui.text_edit_singleline(&mut name);
    if res.changed() {
        *dirty = true;
        palette.name.pop();
        palette.name.push(name);
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

    if *dirty
        && ui
            .button("Save")
            .on_hover_ui(|ui| {
                ui.label("Save the palette to disk");
            })
            .clicked()
    {
        *dirty = false;
        AssetTrait::save(palette.clone());
    }
}
