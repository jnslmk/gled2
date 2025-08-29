use crate::storage::{
    asset::{Asset, AssetTrait},
    asset_id::AssetId,
};
use egui::{Button, Color32, Id, Label, Margin, Pos2, Rect, Stroke, Ui, UiKind, Vec2};
use egui_flex::{Flex, item};
use egui_ltreeview::{Action, DragAndDrop, NodeBuilder, TreeView, TreeViewBuilder};
use std::{collections::BTreeMap, sync::Arc};

pub const TREE_WIDTH: f32 = 300.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeEntry<T: AssetTrait> {
    Dir(Vec<String>, BTreeMap<String, TreeEntry<T>>),
    Asset(Arc<Asset<T>>),
}

impl<T: AssetTrait> Default for TreeEntry<T> {
    fn default() -> Self {
        TreeEntry::Dir(vec![], BTreeMap::new())
    }
}

impl<T: AssetTrait> TreeEntry<T> {
    pub fn build(
        &self,
        empty_dirs: &mut Vec<Vec<String>>,
        tree_ids: &mut Vec<TreeId<T>>,
        builder: &mut TreeViewBuilder<usize>,
        only_asset_selection: bool,
    ) {
        match self {
            TreeEntry::Dir(dir, children) => {
                tree_ids.push(TreeId::Dir(dir.to_owned()));
                builder.node(
                    NodeBuilder::dir(tree_ids.len() - 1)
                        .icon(|ui| {
                            if !only_asset_selection {
                                ui.menu_button("+", |ui| {
                                    let new_dir_name = {
                                        let mut name = dir.clone();
                                        name.push("New Folder".to_string());
                                        name
                                    };
                                    if ui
                                        .add_enabled(
                                            !empty_dirs.contains(&new_dir_name)
                                                && !children.values().any(|child| {
                                                    let TreeEntry::Dir(dir, ..) = child else {
                                                        return false;
                                                    };
                                                    dir == &new_dir_name
                                                }),
                                            Button::new("Folder"),
                                        )
                                        .clicked()
                                    {
                                        empty_dirs.push(new_dir_name);
                                        ui.close_kind(UiKind::Menu);
                                    }

                                    let new_asset_name = {
                                        let mut name = dir.clone();
                                        name.push(format!("New {}", T::NAME));
                                        name
                                    };
                                    if ui
                                        .add_enabled(
                                            !children.values().any(|child| {
                                                let TreeEntry::Asset(asset) = child else {
                                                    return false;
                                                };
                                                asset.path == new_asset_name
                                            }),
                                            Button::new(T::NAME),
                                        )
                                        .clicked()
                                    {
                                        let mut asset = Asset::<T>::new(new_asset_name);
                                        asset.change_dir(dir);
                                        asset.save();
                                        ui.close_kind(UiKind::Menu);
                                    }
                                });
                            }
                        })
                        .label(dir.last().cloned().unwrap_or_default()),
                );
                for entry in children
                    .values()
                    .filter(|entry| !matches!(entry, TreeEntry::Dir(..)))
                {
                    entry.build(empty_dirs, tree_ids, builder, only_asset_selection);
                }
                for entry in children
                    .values()
                    .filter(|entry| !matches!(entry, TreeEntry::Asset(..)))
                {
                    entry.build(empty_dirs, tree_ids, builder, only_asset_selection);
                }
                builder.close_dir();
            }
            TreeEntry::Asset(asset) => {
                tree_ids.push(TreeId::File(asset.id));
                builder.node(NodeBuilder::leaf(tree_ids.len() - 1).label_ui(|ui| {
                    ui.add(Label::new(asset.name()).selectable(false).truncate());

                    let min = Pos2::new(
                        ui.next_widget_position().x + ui.available_width() - 150.0,
                        ui.next_widget_position().y - 8.0,
                    );
                    let max =
                        ui.next_widget_position() + Vec2::new(ui.available_width() - 4.0, 8.0);
                    let rect = Rect::from_min_max(min, max);
                    asset.data.show(ui, rect);
                }));
            }
        }
    }
}

#[derive(Debug)]
pub enum TreeId<T: AssetTrait> {
    Dir(Vec<String>),
    File(AssetId<T>),
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum TreeSelection<T: AssetTrait> {
    #[default]
    None,
    Asset(Asset<T>),
    Dir {
        current: Vec<String>,
        new: Vec<String>,
    },
}

pub struct AssetTree<T: AssetTrait> {
    only_asset_selection: bool,
    folder_dirty: bool,
    empty_dirs: Vec<Vec<String>>,
    selection: TreeSelection<T>,
}

impl<T: AssetTrait> Default for AssetTree<T> {
    fn default() -> Self {
        Self {
            only_asset_selection: false,
            folder_dirty: false,
            empty_dirs: Default::default(),
            selection: Default::default(),
        }
    }
}

impl<T: AssetTrait> AssetTree<T> {
    pub fn load(&self) -> Vec<TreeEntry<T>> {
        let mut root = TreeEntry::default();
        let assets = Asset::all();
        for asset in assets {
            let pos = Self::add_dir(&mut root, asset.dir());
            if let TreeEntry::Dir(_, dir) = pos {
                dir.entry(asset.path.last().cloned().unwrap_or_default())
                    .or_insert_with(|| TreeEntry::Asset(asset));
            } else {
                unreachable!();
            }
        }
        for dir in self.empty_dirs.clone().into_iter() {
            Self::add_dir(&mut root, dir);
        }

        let TreeEntry::Dir(_, dir) = root else {
            unreachable!();
        };
        dir.into_values().collect()
    }

    fn add_dir(root: &mut TreeEntry<T>, dir: Vec<String>) -> &mut TreeEntry<T> {
        let mut pos = root;
        for (i, name) in dir.clone().into_iter().enumerate() {
            if let TreeEntry::Dir(_, children) = pos {
                pos = children.entry(name).or_insert_with(|| {
                    TreeEntry::Dir(dir.iter().take(i + 1).cloned().collect(), BTreeMap::new())
                });
            } else {
                unreachable!();
            }
        }
        pos
    }

    /// Returns true if the selection has changed
    pub fn show(&mut self, ui: &mut Ui, id: Id) -> bool {
        ui.set_clip_rect(ui.max_rect());
        let mut selection_changed = false;
        let entries = self.load();
        let mut tree_ids = vec![];

        if !self.only_asset_selection {
            ui.horizontal(|ui| {
                let new_dir_name = vec!["New Folder".to_string()];
                if ui
                    .add_enabled(
                        !self.empty_dirs.contains(&new_dir_name)
                            && !entries.iter().any(|child| {
                                let TreeEntry::Dir(dir, ..) = child else {
                                    return false;
                                };
                                dir == &new_dir_name
                            }),
                        Button::new("+Folder"),
                    )
                    .clicked()
                {
                    self.empty_dirs.push(new_dir_name);
                }
                if ui
                    .add_enabled(
                        !self.empty_dirs.is_empty(),
                        Button::new("Clear Empty Folders"),
                    )
                    .clicked()
                {
                    if matches!(self.selection, TreeSelection::Dir { .. }) {
                        self.selection = TreeSelection::None;
                    }
                    self.empty_dirs.clear();
                }
            });
        }

        let actions = TreeView::new(id)
            .show(ui, |builder| {
                for entry in entries.iter() {
                    entry.build(
                        &mut self.empty_dirs,
                        &mut tree_ids,
                        builder,
                        self.only_asset_selection,
                    );
                }
            })
            .1;

        for action in actions {
            match action {
                Action::SetSelected(index) => {
                    self.selection = index
                        .first()
                        .copied()
                        .map(|index| tree_ids.remove(index))
                        .and_then(|id| match id {
                            TreeId::File(id) => Asset::get(id)
                                .map(|asset| TreeSelection::Asset(Arc::unwrap_or_clone(asset))),
                            TreeId::Dir(dir) => Some(TreeSelection::Dir {
                                current: dir.clone(),
                                new: dir,
                            }),
                        })
                        .unwrap_or_default();
                    selection_changed = true;
                }
                Action::Move(DragAndDrop { source, target, .. }) => {
                    for source in source {
                        let mut target = target;
                        if source < target {
                            target -= 1;
                        }

                        let source = tree_ids.remove(source);
                        let target = tree_ids.remove(target);

                        if let (TreeId::File(source), TreeId::Dir(target)) = (source, target) {
                            let target = target.clone();
                            if let Some(asset) = Asset::get(source) {
                                let mut asset = Arc::unwrap_or_clone(asset);
                                asset.path = target
                                    .into_iter()
                                    .chain(std::iter::once(asset.path.last().unwrap().clone()))
                                    .collect();
                                asset.save();
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        selection_changed
    }

    pub fn selected(&mut self) -> &mut TreeSelection<T> {
        &mut self.selection
    }

    pub fn common_settings(&mut self, ui: &mut Ui, dirty: &mut bool) -> bool {
        let mut changed = false;

        egui::Frame::NONE
            .inner_margin(Margin::from(6.0))
            .stroke(Stroke::new(1.0, Color32::DARK_GRAY))
            .show(ui, |ui| {
                match &mut self.selection {
                    TreeSelection::Asset(asset) => {
                        ui.label("Name:");
                        let mut name = asset.name().to_string();
                        let res = ui
                            .vertical_centered_justified(|ui| ui.text_edit_singleline(&mut name))
                            .inner;
                        if res.changed() {
                            *dirty = true;
                            asset.path.pop();
                            asset.path.push(name);
                        }
                    }
                    TreeSelection::Dir { .. } => {
                        self.show_folder_editor(ui);
                        return;
                    }
                    _ => {
                        ui.label("Please select an item from the tree");
                        return;
                    }
                }

                ui.add_space(4.0);

                Flex::horizontal().show(ui, |flex| {
                    if flex
                        .add(
                            item().grow(1.0),
                            Button::new(format!("Save{}", if *dirty { "*" } else { "" })),
                        )
                        .on_hover_ui(|ui| {
                            ui.label("Save to disk");
                        })
                        .clicked()
                    {
                        *dirty = false;

                        if let TreeSelection::Asset(asset) = &self.selection {
                            asset.clone().save();
                        }

                        changed = true;
                    }
                    if flex
                        .add(item().grow(1.0), Button::new("Save Copy"))
                        .on_hover_ui(|ui| {
                            ui.label("Save copy to disk");
                        })
                        .clicked()
                    {
                        *dirty = false;

                        if let TreeSelection::Asset(asset) = &self.selection {
                            asset.copy().save();
                        }

                        changed = true;
                    }
                    if flex
                        .add(
                            item().grow(1.0),
                            Button::new(format!("Reset{}", if *dirty { "*" } else { "" }))
                                .fill(Color32::DARK_RED),
                        )
                        .on_hover_ui(|ui| {
                            ui.label("Reset to state on disk");
                        })
                        .clicked()
                    {
                        *dirty = false;
                        if let TreeSelection::Asset(asset) = &mut self.selection {
                            *asset = Arc::unwrap_or_clone(Asset::get(asset.id).unwrap_or_default());
                        }

                        changed = true;
                    }
                    if flex
                        .add(
                            item().grow(1.0),
                            Button::new("Delete").fill(Color32::DARK_RED),
                        )
                        .clicked()
                    {
                        if let TreeSelection::Asset(asset) = &self.selection {
                            asset.delete();
                        }
                        self.selection = TreeSelection::None;

                        changed = true
                    }
                });
            });

        changed
    }

    fn show_folder_editor(&mut self, ui: &mut Ui) {
        let TreeSelection::Dir { current, new } = &mut self.selection else {
            return;
        };

        ui.label("Name:");
        let mut name = new.last().cloned().unwrap_or_default();
        let res = ui
            .vertical_centered_justified(|ui| ui.text_edit_singleline(&mut name))
            .inner;
        if res.changed() {
            self.folder_dirty = true;
            new.pop();
            new.push(name);
        }

        ui.add_enabled_ui(self.folder_dirty, |ui| {
            ui.vertical_centered_justified(|ui| {
                if ui
                    .button("Save")
                    .on_hover_ui(|ui| {
                        ui.label("Save all assets in this folder with new name to disk");
                    })
                    .clicked()
                {
                    self.folder_dirty = false;
                    let assets = Asset::<T>::all();
                    for asset in assets {
                        if &asset.dir() == current {
                            let mut asset = Arc::unwrap_or_clone(asset);
                            asset.change_dir(new);
                            asset.save();
                        }
                    }
                    for empty_dir in self.empty_dirs.iter_mut() {
                        if empty_dir.len() >= current.len()
                            && empty_dir[..current.len()] == current[..]
                        {
                            empty_dir[..current.len()].clone_from_slice(&new[..]);
                        }
                    }
                    *current = new.clone();
                }
            });
        });
    }

    pub fn show_asset_selection(ui: &mut Ui, id: Id) -> Option<AssetId<T>> {
        let mut tree = AssetTree {
            only_asset_selection: true,
            ..Default::default()
        };
        tree.show(ui, id);
        match tree.selection {
            TreeSelection::Asset(asset) => Some(asset.id),
            _ => None,
        }
    }
}
