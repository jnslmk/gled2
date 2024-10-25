use super::{Asset, AssetTrait};
use crate::storage::AssetPath;
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::{collections::HashMap, path::PathBuf, sync::Arc};

#[derive(Debug, Clone)]
pub struct Folder<T: AssetTrait>(HashMap<Arc<String>, Asset<T>>);

impl<T: AssetTrait> Default for Folder<T> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl<T: AssetTrait> Folder<T> {
    pub fn load(path: PathBuf) -> Self {
        let Ok(directory) = path.read_dir() else {
            return Self(HashMap::new());
        };
        let folder = Arc::new(
            path.parent()
                .expect("Path has no parent")
                .to_string_lossy()
                .to_string(),
        );

        let assets = directory
            .par_bridge()
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let file_type = entry.file_type().ok()?;

                if file_type.is_file() {
                    let file = Arc::new(entry.file_name().into_string().ok()?);

                    let path = AssetPath {
                        folder: folder.clone(),
                        file: file.clone(),
                    };
                    let data =
                        serde_json::from_reader(std::fs::File::open(entry.path()).ok()?).ok()?;

                    return Some((file, Asset { path, data }));
                }

                None
            })
            .collect();

        Self(assets)
    }

    pub fn assets(&self) -> Vec<&Asset<T>> {
        self.0.values().collect()
    }

    pub fn get(&self, file: &Arc<String>) -> Option<&Asset<T>> {
        self.0.get(file)
    }

    /// Overwrite asset in memory cache
    pub fn set_asset(&mut self, file: Arc<String>, asset: Asset<T>) {
        self.0.insert(file, asset);
    }
}
