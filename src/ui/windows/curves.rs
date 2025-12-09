use crate::{
    storage::asset::{Asset, curve::Curve},
    ui::{
        asset_tree::{AssetTree, TREE_WIDTH, TreeSelection},
        window_common::{default_viewport_builder, gled_window_frame},
    },
};
use egui::{Context, Id, Margin, Rect, Ui, Vec2, ViewportId};

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

        ctx.show_viewport_immediate(
            ViewportId(Id::new("curves window")),
            default_viewport_builder()
                .with_inner_size(Vec2::new(660.0, 500.0))
                .with_min_inner_size(Vec2::new(660.0, 500.0)),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.open = false;
                    }
                });

                gled_window_frame(ctx, "Curves", |ui| {
                    egui::SidePanel::left("curves tree")
                        .exact_width(TREE_WIDTH)
                        .resizable(false)
                        .show_inside(ui, |ui| {
                            if self.tree.show(ui, ui.make_persistent_id("curves_tree")) {
                                self.dirty = false;
                            }
                        });

                    egui::CentralPanel::default().show_inside(ui, |ui| {
                        self.tree.common_settings(ui, &mut self.dirty);

                        if let TreeSelection::Asset(curve) = &mut self.tree.selected() {
                            curve_editor(ui, curve, &mut self.dirty);
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

fn curve_editor(ui: &mut Ui, curve: &mut Asset<Curve>, dirty: &mut bool) {
    let width = ui.available_width() - 20.0;
    ui.horizontal(|ui| {
        egui::Frame::NONE
            .inner_margin(Margin::from(3.0))
            .show(ui, |ui| {
                ui.set_max_width(width / 2.0);
                ui.vertical_centered_justified(|ui| {
                    if ui.button("Invert X-Axis ↔").clicked() {
                        curve.data.invert_x_axis();
                        *dirty = true;
                    }
                });
            });
        egui::Frame::NONE
            .inner_margin(Margin::from(3.0))
            .show(ui, |ui| {
                ui.set_max_width(width / 2.0);
                ui.vertical_centered_justified(|ui| {
                    if ui.button("Invert Y-Axis ↕").clicked() {
                        curve.data.invert_y_axis();
                        *dirty = true;
                    }
                });
            });
    });

    *dirty |= curve.data.draw(
        ui,
        true,
        None,
        Rect::from_min_size(ui.next_widget_position(), ui.available_size()),
    );
}
