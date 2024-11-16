use crate::{
    storage::{Asset, Palette},
    ui::asset_tree::{AssetTree, TreeSelection},
};
use egui::{Context, Margin, Ui};

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

                egui::Frame::default()
                    .outer_margin(Margin::same(4.0))
                    .show(ui, |ui| {
                        self.tree.common_settings(ui, &mut self.dirty);

                        if let TreeSelection::Asset(palette) = &mut self.tree.selected() {
                            palette_editor(ui, palette, &mut self.dirty);
                        }
                    });
            });
    }

    pub fn open(&mut self) {
        self.open = true;
    }
}

fn palette_editor(ui: &mut Ui, palette: &mut Asset<Palette>, dirty: &mut bool) {
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
}
