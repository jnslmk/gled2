use crate::{
    storage::{Asset, AssetTrait, Palette},
    ui::asset_tree::{AssetTree, TreeSelection},
};
use egui::{Context, Margin, Ui};

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
