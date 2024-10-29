use crate::storage::{Asset, AssetId, AssetTrait};
use egui::{Align, Button, Color32, Label, Layout, Stroke, Ui};
use egui_ltreeview::{node::NodeBuilder, Action, TreeView, TreeViewBuilder};
use std::{collections::BTreeMap, sync::Arc};

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
    ) {
        match self {
            TreeEntry::Dir(dir, children) => {
                tree_ids.push(TreeId::Dir(dir.to_owned()));
                builder.node(
                    NodeBuilder::dir(tree_ids.len() - 1)
                        .icon(|ui| {
                            ui.menu_button("+", |ui| {
                                if ui.button("Folder").clicked() {
                                    let mut dir = dir.clone();
                                    dir.push("New Folder".to_string());
                                    if !empty_dirs.contains(&dir) {
                                        empty_dirs.push(dir);
                                    }
                                    ui.close_menu();
                                }
                                if ui.button("Asset").clicked() {
                                    let mut name = dir.clone();
                                    name.push("New Asset".to_string());
                                    let mut asset = Asset::<T>::new(name);
                                    asset.change_dir(dir);
                                    AssetTrait::save(asset);
                                    ui.close_menu();
                                }
                            });
                        })
                        .label(|ui| {
                            ui.add(
                                Label::new(dir.last().cloned().unwrap_or_default())
                                    .selectable(false),
                            );
                        }),
                );
                for entry in children
                    .values()
                    .filter(|entry| !matches!(entry, TreeEntry::Dir(..)))
                {
                    entry.build(empty_dirs, tree_ids, builder);
                }
                for entry in children
                    .values()
                    .filter(|entry| !matches!(entry, TreeEntry::Asset(..)))
                {
                    entry.build(empty_dirs, tree_ids, builder);
                }
                builder.close_dir();
            }
            TreeEntry::Asset(asset) => {
                tree_ids.push(TreeId::File(asset.id));
                builder.node(NodeBuilder::leaf(tree_ids.len() - 1).label(|ui| {
                    ui.add(
                        Label::new(asset.name.last().cloned().unwrap_or_default())
                            .selectable(false),
                    );
                    asset.data.tree_entry_show(ui);
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

#[derive(Debug, Default)]
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
    folder_dirty: bool,
    empty_dirs: Vec<Vec<String>>,
    selection: TreeSelection<T>,
}

impl<T: AssetTrait> Default for AssetTree<T> {
    fn default() -> Self {
        Self {
            folder_dirty: false,
            empty_dirs: Default::default(),
            selection: Default::default(),
        }
    }
}

impl<T: AssetTrait> AssetTree<T> {
    pub fn load(&self) -> Vec<TreeEntry<T>> {
        let mut root = TreeEntry::default();
        let assets = T::all();
        for asset in assets {
            let pos = Self::add_dir(&mut root, asset.dir());
            if let TreeEntry::Dir(_, ref mut dir) = pos {
                dir.entry(asset.name.last().cloned().unwrap_or_default())
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
            if let TreeEntry::Dir(_, ref mut children) = pos {
                pos = children.entry(name).or_insert_with(|| {
                    TreeEntry::Dir(dir.iter().take(i + 1).cloned().collect(), BTreeMap::new())
                });
            } else {
                unreachable!();
            }
        }
        pos
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui.button("+Folder").clicked() {
                let dir = vec!["New Folder".to_string()];
                if !self.empty_dirs.contains(&dir) {
                    self.empty_dirs.push(dir);
                }
            }
            if ui.button("Clear Empty Folders").clicked() {
                self.empty_dirs.clear();
            }
        });

        let entries = self.load();
        let mut tree_ids = vec![];

        let actions = TreeView::new(ui.make_persistent_id("asset tree view"))
            .show(ui, |mut builder| {
                for entry in entries.iter() {
                    entry.build(&mut self.empty_dirs, &mut tree_ids, &mut builder);
                }
            })
            .actions;

        for action in actions {
            match action {
                Action::SetSelected(index) => {
                    self.selection = index
                        .map(|index| tree_ids.remove(index))
                        .map(|id| match id {
                            TreeId::File(id) => {
                                TreeSelection::Asset(Arc::unwrap_or_clone(T::get(id)))
                            }
                            TreeId::Dir(dir) => TreeSelection::Dir {
                                current: dir.clone(),
                                new: dir,
                            },
                        })
                        .unwrap_or_default();
                }
                Action::Move {
                    source, mut target, ..
                } => {
                    if source < target {
                        target -= 1;
                    }

                    let source = tree_ids.remove(source);
                    let target = tree_ids.remove(target);

                    if let (TreeId::File(source), TreeId::Dir(target)) = (source, target) {
                        let target = target.clone();
                        let mut asset = Arc::unwrap_or_clone(T::get(source));
                        asset.name = target
                            .into_iter()
                            .chain(std::iter::once(asset.name.last().unwrap().clone()))
                            .collect();
                        AssetTrait::save(asset);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn selected(&mut self) -> &mut TreeSelection<T> {
        &mut self.selection
    }

    pub fn show_delete_button(&mut self, ui: &mut Ui) {
        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
            if let TreeSelection::Asset(asset) = &self.selection {
                if ui
                    .add(Button::new("Delete").fill(Color32::DARK_RED))
                    .clicked()
                {
                    AssetTrait::delete(asset.id);
                    self.selection = TreeSelection::None;
                }
            }
        });
    }

    pub fn show_folder_editor(&mut self, ui: &mut Ui) {
        let TreeSelection::Dir { current, new } = &mut self.selection else {
            return;
        };

        ui.label("Name:");
        let mut name = new.last().cloned().unwrap_or_default();
        let res = ui.text_edit_singleline(&mut name);
        if res.changed() {
            self.folder_dirty = true;
            new.pop();
            new.push(name);
        }

        if self.folder_dirty
            && ui
                .button("Save")
                .on_hover_ui(|ui| {
                    ui.label("Save all assets in this folder with new name to disk");
                })
                .clicked()
        {
            self.folder_dirty = false;
            let assets = T::all();
            for asset in assets {
                if &asset.dir() == current {
                    let mut asset = Arc::unwrap_or_clone(asset);
                    asset.change_dir(new);
                    AssetTrait::save(asset);
                }
            }
            for empty_dir in self.empty_dirs.iter_mut() {
                if empty_dir.len() > current.len() && empty_dir[..current.len()] == current[..] {
                    empty_dir[..current.len()].clone_from_slice(&new[..]);
                }
            }
            *current = new.clone();
        }
    }
}
