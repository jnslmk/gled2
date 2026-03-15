use crate::storage::{
    asset::{Asset, AssetTrait},
    asset_id::AssetId,
    collections::Collections,
};
use egui::{
    Button, Color32, Id, Label, Margin, Pos2, Rect, ScrollArea, Stroke, Ui, UiKind, Vec2,
    scroll_area::ScrollBarVisibility::AlwaysVisible,
};
use egui_ltreeview::{Action, DragAndDrop, NodeBuilder, TreeView, TreeViewBuilder};
use std::{collections::BTreeMap, sync::Arc};

mod ui_methods;

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
        builder: &mut Option<&mut TreeViewBuilder<usize>>,
        only_asset_selection: bool,
        collections: &mut Collections,
    ) {
        match self {
            TreeEntry::Dir(dir, children) => {
                tree_ids.push(TreeId::Dir(dir.to_owned()));
                if let Some(builder) = builder {
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
                                            asset.save(collections);
                                            ui.close_kind(UiKind::Menu);
                                        }
                                    });
                                }
                            })
                            .label(dir.last().cloned().unwrap_or_default()),
                    );
                }

                for entry in children
                    .values()
                    .filter(|entry| !matches!(entry, TreeEntry::Dir(..)))
                {
                    entry.build(
                        empty_dirs,
                        tree_ids,
                        builder,
                        only_asset_selection,
                        collections,
                    );
                }
                for entry in children
                    .values()
                    .filter(|entry| !matches!(entry, TreeEntry::Asset(..)))
                {
                    entry.build(
                        empty_dirs,
                        tree_ids,
                        builder,
                        only_asset_selection,
                        collections,
                    );
                }

                if let Some(builder) = builder {
                    builder.close_dir();
                }
            }
            TreeEntry::Asset(asset) => {
                tree_ids.push(TreeId::File(asset.id));
                if let Some(builder) = builder {
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
}

#[derive(Debug, PartialEq)]
pub enum TreeId<T: AssetTrait + PartialEq> {
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
    pub only_asset_selection: bool,
    pub folder_dirty: bool,
    pub empty_dirs: Vec<Vec<String>>,
    pub selection: TreeSelection<T>,
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
    pub fn load(&self, collections: &Collections) -> Vec<TreeEntry<T>> {
        let mut root = TreeEntry::default();
        let assets = Asset::all(collections);
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

    pub fn find_index(&self, id: &TreeId<T>, collections: &mut Collections) -> Option<usize> {
        let mut empty_dirs = vec![];
        let entries = self.load(collections);
        let mut tree_ids = vec![];
        for entry in entries.iter() {
            entry.build(
                &mut empty_dirs,
                &mut tree_ids,
                &mut None,
                self.only_asset_selection,
                collections,
            );
        }

        tree_ids.iter().position(|tree_id| tree_id == id)
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
}
