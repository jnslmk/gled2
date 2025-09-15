use crate::{
    storage::asset::{Asset, palette::Palette},
    ui::{
        asset_tree::{AssetTree, TREE_WIDTH, TreeSelection},
        window_common::{default_viewport_builder, gled_window_frame},
    },
};
use egui::{Context, Id, Ui, Vec2, ViewportId};

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

        ctx.show_viewport_immediate(
            ViewportId(Id::new("palettes window")),
            default_viewport_builder()
                .with_inner_size(Vec2::new(650.0, 500.0))
                .with_min_inner_size(Vec2::new(650.0, 500.0)),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.open = false;
                    }
                });

                gled_window_frame(ctx, "Palettes", |ui| {
                    egui::SidePanel::left("palettes tree")
                        .exact_width(TREE_WIDTH)
                        .resizable(false)
                        .show_inside(ui, |ui| {
                            if self.tree.show(ui, ui.make_persistent_id("palettes_tree")) {
                                self.dirty = false;
                            }
                        });

                    egui::CentralPanel::default().show_inside(ui, |ui| {
                        self.tree.common_settings(ui, &mut self.dirty);

                        if let TreeSelection::Asset(palette) = &mut self.tree.selected() {
                            palette_editor(ui, palette, &mut self.dirty);
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

fn palette_editor(ui: &mut Ui, palette: &mut Asset<Palette>, dirty: &mut bool) {
    ui.label("Primary color:");
    let res = ui.color_edit_button_rgb(palette.data.primary.rgb_mut());
    *dirty |= res.changed();

    ui.label("Secondary color:");
    let res = ui.color_edit_button_rgb(palette.data.secondary.rgb_mut());
    *dirty |= res.changed();

    ui.label("Gradient colors:");
    ui.scope(|ui| {
        palette.data.gradient.chunks_mut(4).for_each(|chunk| {
            ui.horizontal(|ui| {
                chunk.iter_mut().for_each(|color| {
                    let res = ui.color_edit_button_rgb(color.rgb_mut());
                    *dirty |= res.changed();
                });
            });
        });
    });
}
