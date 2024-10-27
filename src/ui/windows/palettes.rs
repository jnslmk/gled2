use crate::storage::{Asset, AssetTrait, Palette};
use egui::{Context, Mesh, Rect, Shape, Ui, Vec2};
use egui_ltreeview::{node::NodeBuilder, TreeView};
use svgdom::Node;

#[derive(Default)]
pub struct PalettesWindow {
    pub open: bool,
    editing: Option<Asset<Palette>>,
}

impl PalettesWindow {
    pub fn update(&mut self, ctx: &Context) {
        if !self.open {
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
                    .default_width(300.0)
                    .min_width(300.0)
                    .show_inside(ui, |ui| {
                        TreeView::new(ui.make_persistent_id("palettes tree view")).show(
                            ui,
                            |mut builder| {
                                let mut palettes = Palette::all();
                                palettes.sort_by_key(|palette| palette.name.clone());

                                let mut id = 0;
                                let mut dir = vec![];
                                for palette in palettes {
                                    let new_dir = palette.dir();
                                    if new_dir != dir {
                                        let mut close_all = false;
                                        let mut skip = 0;
                                        for (pos, name) in dir.iter().enumerate() {
                                            if close_all || new_dir.get(pos) != Some(name) {
                                                builder.close_dir();
                                                close_all = true;
                                            } else {
                                                skip = pos + 1;
                                            }
                                        }
                                        for name in new_dir.iter().skip(skip) {
                                            builder.dir(id, name);
                                            id += 1;
                                        }
                                        dir = new_dir;
                                    }
                                    builder.node(NodeBuilder::leaf(id).label(|ui| {
                                        ui.label(palette.name.last().cloned().unwrap_or_default());
                                        let max = ui.next_widget_position()
                                            + Vec2::new(ui.available_width(), 0.0);
                                        let color_band_rect = Rect::from_min_max(
                                            ui.next_widget_position()
                                                .max(max - Vec2::new(200.0, 0.0)),
                                            max,
                                        );
                                        ui.painter().add(Shape::Mesh(
                                            palette.data.color_band_mesh(color_band_rect),
                                        ));
                                    }));
                                    id += 1;
                                }
                                for _ in dir.iter() {
                                    builder.close_dir();
                                }
                            },
                        );
                    });
                ui.label("test2");
            });
    }

    pub fn open(&mut self) {
        self.open = true;
    }
}

pub fn palette_editor(ui: &mut Ui, palette: &mut Option<Asset<Palette>>) {
    let mut changed = false;

    if let Some(palette) = palette {
        ui.scope(|ui| {
            ui.horizontal_wrapped(|ui| {
                palette.data.gradient.iter_mut().for_each(|color| {
                    let res = ui.color_edit_button_rgb(color.rgb_mut());
                    if res.changed() {
                        changed = true;
                    }
                });
            });
        });
    }

    if changed
        && ui
            .button("Save")
            .on_hover_ui(|ui| {
                ui.label("Save the palette to disk");
            })
            .clicked()
    {
        if let Some(palette) = palette.take() {
            palette.save();
        }
    }
}
