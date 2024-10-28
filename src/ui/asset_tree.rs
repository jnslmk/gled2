use crate::storage::{Asset, AssetId, AssetTrait};
use egui::{Label, Ui};
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
    pub fn build(&self, tree_ids: &mut Vec<TreeId<T>>, builder: &mut TreeViewBuilder<usize>) {
        match self {
            TreeEntry::Dir(dir, children) => {
                tree_ids.push(TreeId::Dir(dir.to_owned()));
                builder.node(NodeBuilder::dir(tree_ids.len() - 1).label(|ui| {
                    ui.add(Label::new(dir.last().cloned().unwrap_or_default()).selectable(false));
                }));
                for entry in children.values() {
                    entry.build(tree_ids, builder);
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
    empty_dirs: Vec<Vec<String>>,
    selection: TreeSelection<T>,
}

impl<T: AssetTrait> Default for AssetTree<T> {
    fn default() -> Self {
        Self {
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

    pub fn add_empty_dir(&mut self, dir: Vec<String>) {
        self.empty_dirs.push(dir.clone());
    }

    pub fn show(&mut self, ui: &mut Ui) {
        let entries = self.load();
        let mut tree_ids = vec![];

        let actions = TreeView::new(ui.make_persistent_id("palettes tree view"))
            .show(ui, |mut builder| {
                for entry in entries.iter() {
                    entry.build(&mut tree_ids, &mut builder);
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
                                TreeSelection::Asset(Arc::unwrap_or_clone(T::get(&id)))
                            }
                            TreeId::Dir(dir) => TreeSelection::Dir {
                                current: dir.clone(),
                                new: dir,
                            },
                        })
                        .unwrap_or_default();
                }
                Action::Move { source, target, .. } => {
                    let source = tree_ids.remove(source);
                    let target = tree_ids.remove(target);
                    if let (TreeId::File(source), TreeId::Dir(target)) = (source, target) {
                        let target = target.clone();
                        let mut asset = Arc::unwrap_or_clone(T::get(&source));
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
}
