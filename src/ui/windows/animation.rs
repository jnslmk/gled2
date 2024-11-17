use crate::{
    app::Timing,
    storage::Animation,
    ui::asset_tree::{AssetTree, TreeSelection, TREE_WIDTH},
};
use egui::{Context, Margin};

#[derive(Default)]
pub struct AnimationWindow {
    open: bool,
    dirty: bool,
    tree: AssetTree<Animation>,
}

impl AnimationWindow {
    pub fn update(&mut self, ctx: &Context, timing: &Timing) {
        /* if !self.open {
            self.dirty = false;
            return;
        } */

        egui::Window::new("Animations")
            .collapsible(false)
            .min_width(800.0)
            .resizable(true)
            .default_pos(ctx.available_rect().center())
            .open(&mut self.open)
            .show(ctx, |ui| {
                egui::SidePanel::left("animations tree")
                    .exact_width(TREE_WIDTH)
                    .resizable(false)
                    .show_inside(ui, |ui| {
                        self.tree.show(ui, ui.make_persistent_id("animations_tree"));
                    });

                egui::SidePanel::right("scene editor")
                    .exact_width(300.0)
                    .resizable(false)
                    .show_inside(ui, |ui| {
                        if let TreeSelection::Asset(animation) = &mut self.tree.selected() {
                            self.dirty |= animation.data.change_arguments_ui(ui);
                        }
                    });

                egui::Frame::default()
                    .outer_margin(Margin::same(4.0))
                    .show(ui, |ui| {
                        self.tree.common_settings(ui, &mut self.dirty);

                        if let TreeSelection::Asset(animation) = &mut self.tree.selected() {
                            self.dirty |= animation.data.change_shader_code_ui(ui);
                        }
                    });
            });
    }

    pub fn open(&mut self) {
        self.open = true;
    }
}
