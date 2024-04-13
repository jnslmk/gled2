use crate::{animation::ColorPalette, effect::Effect, scene::Scene};
use rayon::iter::{ParallelBridge, ParallelIterator};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::HashMap,
    fmt::Debug,
    path::{Path, PathBuf},
};

pub trait AssetTrait: Serialize + Send + DeserializeOwned + Debug {}

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

    pub fn reload(&mut self, root: &Path) {
        *self = Self::load(self.path.clone(), root);
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
}

#[derive(Debug, Clone)]
pub struct Asset<T: AssetTrait> {
    pub path: PathBuf,
    pub data: T,
}
