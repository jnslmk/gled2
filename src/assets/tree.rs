use crate::{effect::Effect, scene::Scene};

use super::Action;
use rayon::iter::{ParallelBridge, ParallelIterator};
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::HashMap, fmt::Debug, path::PathBuf};

pub trait AssetTrait: Serialize + Send + DeserializeOwned + Debug {}

impl AssetTrait for Effect {}
impl AssetTrait for Scene {}

#[derive(Debug, Clone)]
pub struct Tree<T: AssetTrait> {
    path: PathBuf,
    pub assets: HashMap<String, Asset<T>>,
    pub children: HashMap<String, Tree<T>>,
}

///TODO: All paths must be relative to root!
impl<T: AssetTrait> Tree<T> {
    pub fn load(path: PathBuf) -> Self {
        let mut children = HashMap::new();
        let mut assets = HashMap::new();

        let Ok(paths) = path.read_dir() else {
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

                if file_type.is_dir() {
                    return Some((file_name, Entry::Directory(Tree::load(entry.path()))));
                }

                if file_type.is_file() {
                    let file = std::fs::File::open(entry.path()).ok()?;
                    let data = serde_json::from_reader(file).ok()?;

                    return Some((
                        file_name,
                        Entry::Asset(Asset {
                            path: entry.path(),
                            data,
                        }),
                    ));
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
}

#[derive(Debug, Clone)]
pub struct Asset<T: AssetTrait> {
    path: PathBuf,
    pub data: T,
}

impl<T: AssetTrait> Asset<T> {
    pub fn update(&self, data: T) {
        let contents = serde_json::to_vec_pretty(&data).expect("Could not serialize data");

        Action::SaveFile {
            path: self.path.clone(),
            contents,
            message: format!("Update asset {}", self.path.display()),
        }
        .send();
    }
}
