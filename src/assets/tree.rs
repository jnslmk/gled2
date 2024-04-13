use crate::{animation::ColorPalette, effect::Effect, scene::Scene};
use rayon::iter::{ParallelBridge, ParallelIterator};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::HashMap,
    fmt::Debug,
    path::{Path, PathBuf},
    sync::Arc,
};

pub trait AssetTrait: Serialize + DeserializeOwned + Debug + Send + Sync {}

impl AssetTrait for Effect {}
impl AssetTrait for Scene {}
impl AssetTrait for ColorPalette {}

#[derive(Debug, Clone)]
pub struct Tree<T: AssetTrait> {
    pub path: PathBuf,
    pub assets: HashMap<String, Asset<T>>,
    pub children: HashMap<String, Tree<T>>,
}

impl<T: AssetTrait> Tree<T> {
    pub fn load(path: PathBuf, root: &Path) -> Self {
        let mut children = HashMap::new();
        let mut assets = HashMap::new();

        let Ok(paths) = root.join(&path).read_dir() else {
            return Self {
                path,
                assets,
                children,
            };
        };

        enum Entry<T: AssetTrait> {
            Directory(Tree<T>),
            Asset(Asset<T>),
        }

        let entries = paths
            .par_bridge()
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let file_name = entry.file_name().into_string().ok()?;
                let file_type = entry.file_type().ok()?;
                let path = entry.path();
                let path = path
                    .strip_prefix(root)
                    .expect("entry.path does not have root prefix")
                    .to_path_buf();

                if file_type.is_dir() {
                    return Some((file_name, Entry::Directory(Tree::load(path, root))));
                }

                if file_type.is_file() {
                    let file = std::fs::File::open(entry.path()).ok()?;
                    let data = serde_json::from_reader(file).ok()?;

                    return Some((file_name, Entry::Asset(Asset { path, data })));
                }

                None
            })
            .collect::<Vec<_>>();

        for (file_name, entry) in entries {
            match entry {
                Entry::Directory(directory) => {
                    children.insert(file_name, directory);
                }
                Entry::Asset(asset) => {
                    assets.insert(file_name, asset);
                }
            }
        }

        Self {
            path,
            assets,
            children,
        }
    }

    /// find all assets in the tree
    pub fn all_assets(&self) -> Vec<&Asset<T>> {
        self.assets
            .values()
            .chain(
                self.children
                    .values()
                    .flat_map(|tree| tree.all_assets().into_iter()),
            )
            .collect()
    }

    pub fn find(&self, path: &Path) -> Option<&Asset<T>> {
        let mut position = self;
        for component in path.components() {
            let name = component.as_os_str().to_str()?;
            if let Some(child) = position.children.get(name) {
                position = child;
                continue;
            }
            if let Some(asset) = position.assets.get(name) {
                return Some(asset);
            }
        }

        None
    }

    /// Overwrite asset in memory cache
    pub fn set_cache(&mut self, path: PathBuf, data: Arc<T>) {
        let mut tree = self;
        let Some(parent) = path.parent() else {
            log::error!("Could not get parent of path: {}", path.display());
            return;
        };
        for component in parent.components() {
            let Some(name) = component.as_os_str().to_str() else {
                log::error!("Could not convert path component to string: {component:?}");
                return;
            };

            tree = tree
                .children
                .entry(name.to_owned())
                .or_insert_with(|| Tree {
                    path: tree.path.join(name),
                    assets: HashMap::new(),
                    children: HashMap::new(),
                });
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            log::error!("Could not get file name of path: {}", path.display());
            return;
        };
        tree.assets
            .insert(file_name.to_owned(), Asset { path, data });
    }
}

#[derive(Debug, Clone)]
pub struct Asset<T: AssetTrait> {
    pub path: PathBuf,
    pub data: Arc<T>,
}
