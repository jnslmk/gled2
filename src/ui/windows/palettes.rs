use crate::storage::{Asset, AssetId, AssetTrait, Palette};
use egui::{Context, Label, Margin, Rect, Shape, Ui, Vec2};
use egui_ltreeview::{node::NodeBuilder, Action, TreeView};
use std::sync::Arc;

#[derive(Debug, Default)]
#[allow(clippy::large_enum_variant)]
enum Selection {
    #[default]
    None,
    Palette(Asset<Palette>),
    Dir {
        current: Vec<String>,
        new: Vec<String>,
    },
}

#[derive(Default)]
pub struct PalettesWindow {
    pub open: bool,
    selection: Selection,
    dirty: bool,
}

impl PalettesWindow {
    pub fn update(&mut self, ctx: &Context) {
        if !self.open {
            self.selection = Selection::None;
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
                        enum TreeId {
                            Dir(Vec<String>),
                            File(AssetId<Palette>),
                        }
                        let mut tree_ids = vec![];

                        let actions = TreeView::new(ui.make_persistent_id("palettes tree view"))
                            .show(ui, |mut builder| {
                                let mut palettes = Palette::all();
                                palettes.sort_by_key(|palette| palette.name.clone());

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
                                        dir = new_dir.iter().take(skip).cloned().collect();
                                        for name in new_dir.iter().skip(skip) {
                                            dir.push(name.clone());
                                            tree_ids.push(TreeId::Dir(dir.clone()));
                                            builder.dir(tree_ids.len() - 1, name);
                                        }
                                    }

                                    tree_ids.push(TreeId::File(palette.id));
                                    builder.node(NodeBuilder::leaf(tree_ids.len() - 1).label(
                                        |ui| {
                                            ui.add(
                                                Label::new(
                                                    palette
                                                        .name
                                                        .last()
                                                        .cloned()
                                                        .unwrap_or_default(),
                                                )
                                                .selectable(false),
                                            );
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
                                        },
                                    ));
                                }
                                for _ in dir.iter() {
                                    builder.close_dir();
                                }
                            })
                            .actions;

                        for action in actions {
                            match action {
                                Action::SetSelected(index) => {
                                    self.dirty = false;
                                    self.selection = index
                                        .map(|index| tree_ids.remove(index))
                                        .map(|id| match id {
                                            TreeId::File(id) => Selection::Palette(
                                                Arc::unwrap_or_clone(Palette::get(&id)),
                                            ),
                                            TreeId::Dir(dir) => Selection::Dir {
                                                current: dir.clone(),
                                                new: dir,
                                            },
                                        })
                                        .unwrap_or_default();
                                }
                                Action::Move { source, target, .. } => {
                                    let source = tree_ids.remove(source);
                                    let target = tree_ids.remove(target);
                                    if let (TreeId::File(source), TreeId::Dir(target)) =
                                        (source, target)
                                    {
                                        let target = target.clone();
                                        let mut palette =
                                            Arc::unwrap_or_clone(Palette::get(&source));
                                        palette.name = target
                                            .into_iter()
                                            .chain(std::iter::once(
                                                palette.name.last().unwrap().clone(),
                                            ))
                                            .collect();
                                        palette.save();
                                    }
                                }
                                _ => {}
                            }
                        }
                    });

                egui::Frame::default().outer_margin(Margin::same(4.0)).show(
                    ui,
                    |ui| match &mut self.selection {
                        Selection::None => {
                            ui.label("Please select an item from the tree");
                        }
                        Selection::Palette(palette) => {
                            palette_editor(ui, palette, &mut self.dirty);
                        }
                        Selection::Dir { current, new } => {
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
                palette.save();
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
        palette.clone().save();
    }
}
