use crate::{
    storage::asset::{Asset, curve::Curve},
    ui::{
        asset_tree::{AssetTree, TREE_WIDTH, TreeSelection},
        viewport_builder::default_viewport_builder,
    },
};
use egui::{Button, Context, Id, Rect, Ui, Vec2, ViewportId};
use egui_flex::{Flex, item};

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
                .with_title("Gled: Curves")
                .with_inner_size(Vec2::new(600.0, 500.0))
                .with_min_inner_size(Vec2::new(600.0, 500.0)),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.open = false;
                    }
                });

                egui::SidePanel::left("curves tree")
                    .exact_width(TREE_WIDTH)
                    .resizable(false)
                    .show(ctx, |ui| {
                        if self.tree.show(ui, ui.make_persistent_id("curves_tree")) {
                            self.dirty = false;
                        }
                    });

                egui::CentralPanel::default().show(ctx, |ui| {
                    self.tree.common_settings(ui, &mut self.dirty);

                    if let TreeSelection::Asset(curve) = &mut self.tree.selected() {
                        curve_editor(ui, curve, &mut self.dirty);
                    }
                });
            },
        );
    }

    pub fn open(&mut self) {
        self.open = true;
    }
}

fn curve_editor(ui: &mut Ui, curve: &mut Asset<Curve>, dirty: &mut bool) {
    Flex::horizontal().show(ui, |flex| {
        if flex
            .add(item().grow(1.0), Button::new("Invert X-Axis ↔".to_string()))
            .clicked()
        {
            curve.data.invert_x_axis();
            *dirty = true;
        }
        if flex
            .add(item().grow(1.0), Button::new("Invert Y-Axis ↕".to_string()))
            .clicked()
        {
            curve.data.invert_y_axis();
            *dirty = true;
        }
    });

    *dirty |= curve.data.draw(
        ui,
        true,
        Rect::from_min_size(ui.next_widget_position(), ui.available_size()),
    );
}
