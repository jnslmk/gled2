use super::{folder::Folder, Asset, AssetTrait};
use crate::storage::AssetPath;
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::{collections::HashMap, fmt::Debug, path::PathBuf, sync::Arc};

#[derive(Debug, Clone, Default)]
pub struct Folders<T: AssetTrait>(HashMap<Arc<String>, Folder<T>>);

impl<T: AssetTrait> Folders<T> {
    pub fn load(path: PathBuf) -> Self {
        let Ok(directory) = path.read_dir() else {
            return Self(HashMap::new());
        };

        let folders = directory
            .par_bridge()
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let file_type = entry.file_type().ok()?;

                if file_type.is_dir() {
                    let name = Arc::new(entry.file_name().into_string().ok()?);
                    let folder = Folder::load(entry.path());

                    return Some((name, folder));
                }

                None
            })
            .collect();

        Self(folders)
    }

    /// find all assets in the tree
    pub fn all_assets(&self) -> Vec<&Asset<T>> {
        self.0
            .values()
            .flat_map(|folder| folder.assets().into_iter())
            .collect()
    }

    pub fn get(&self, path: &AssetPath) -> Option<&Asset<T>> {
        self.0.get(&path.folder)?.get(&path.file)
    }

    /// Overwrite asset in memory cache
    pub fn set_asset(&mut self, asset: Asset<T>) {
        let path = asset.path.clone();
        self.0
            .entry(path.folder)
            .or_default()
            .set_asset(path.file, asset);
    }
}
