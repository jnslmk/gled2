use super::asset::{Asset, AssetTrait};
use crate::storage::AssetId;
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::{collections::HashMap, fmt::Debug, path::PathBuf, str::FromStr, sync::Arc};
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct Collection<T: AssetTrait>(HashMap<Uuid, Arc<Asset<T>>>);

impl<T: AssetTrait> Collection<T> {
    pub fn load(path: PathBuf) -> Self {
        let Ok(directory) = path.read_dir() else {
            return Self(HashMap::new());
        };

        let folders = directory
            .par_bridge()
            .filter_map(|entry| {
                let entry = entry
                    .map_err(|err| log::warn!("Could not read dir entry: {err:?}"))
                    .ok()?;
                let file_type = entry
                    .file_type()
                    .map_err(|err| log::warn!("Could not get file type: {err:?}"))
                    .ok()?;

                if file_type.is_file() {
                    let id = Uuid::from_str(
                        entry
                            .file_name()
                            .into_string()
                            .map_err(|err| {
                                log::warn!("Could not convert file name to string: {err:?}")
                            })
                            .ok()?
                            .strip_suffix(".json")?,
                    )
                    .map_err(|err| log::warn!("Could not parse UUID: {err:?}"))
                    .ok()?;

                    let asset =
                        Asset::<T>::read(std::fs::File::open(entry.path()).ok()?, AssetId::new(id))
                            .map_err(|err| log::warn!("Could not read/parse asset: {err:?}"))
                            .ok()?;

                    return Some((id, asset));
                }

                None
            })
            .collect();

        Self(folders)
    }

    /// returns all assets in the collection
    pub fn assets(&self) -> Vec<&Arc<Asset<T>>> {
        self.0.values().collect()
    }

    pub fn get(&self, id: &AssetId<T>) -> Option<&Arc<Asset<T>>> {
        self.0.get(&id.id)
    }

    /// Overwrite asset in memory cache
    pub fn set_asset(&mut self, asset: Asset<T>) {
        self.0.insert(asset.id.id, Arc::new(asset));
    }
}
