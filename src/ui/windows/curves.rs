use crate::{
    storage::{Asset, Curve},
    ui::asset_tree::{AssetTree, TreeSelection, TREE_WIDTH},
};
use egui::{Context, Margin, Ui};

#[derive(Default)]
pub struct CurvesWindow {
    open: bool,
    dirty: bool,
    tree: AssetTree<Curve>,
}

impl CurvesWindow {
    pub fn update(&mut self, ctx: &Context) {
        if !self.open {
            self.dirty = false;
            return;
        }

        egui::Window::new("Curves")
            .collapsible(false)
            .min_width(500.0)
            .resizable(true)
            .default_pos(ctx.available_rect().center())
            .open(&mut self.open)
            .show(ctx, |ui| {
                egui::SidePanel::left("curves tree")
                    .exact_width(TREE_WIDTH)
                    .resizable(false)
                    .show_inside(ui, |ui| {
                        self.tree.show(ui, ui.make_persistent_id("curves_tree"));
                    });

                egui::Frame::default()
                    .outer_margin(Margin::same(4.0))
                    .show(ui, |ui| {
                        self.tree.common_settings(ui, &mut self.dirty);

                        if let TreeSelection::Asset(curve) = &mut self.tree.selected() {
                            curve_editor(ui, curve, &mut self.dirty);
                        }
                    });
            });
    }

    pub fn open(&mut self) {
        self.open = true;
    }
}

fn curve_editor(ui: &mut Ui, curve: &mut Asset<Curve>, dirty: &mut bool) {
    *dirty |= curve.data.draw(ui, true);
}
