use super::{
    STORAGE_DIR,
    asset::{Asset, AssetTrait},
};
use crate::{storage::AssetId, ui::action::UiAction};
use std::{collections::BTreeMap, fmt::Debug, str::FromStr, sync::Arc};
use typemap::Key;
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct Collection<T: AssetTrait>(BTreeMap<Uuid, Arc<Asset<T>>>);

impl<T: AssetTrait + 'static> Key for Collection<T> {
    type Value = Collection<T>;
}

impl<T: AssetTrait> Collection<T> {
    pub fn load() -> Self {
        let path = STORAGE_DIR.join(T::DIR_NAME);
        let Ok(directory) = path.read_dir() else {
            return Self(BTreeMap::new());
        };

        let folders = directory
            .into_iter()
            .filter_map(|entry| {
                let entry = entry
                    .map_err(|err| tracing::warn!("Could not read dir entry: {err:?}"))
                    .ok()?;
                let file_type = entry
                    .file_type()
                    .map_err(|err| tracing::warn!("Could not get file type: {err:?}"))
                    .ok()?;

                if file_type.is_file() {
                    let id = Uuid::from_str(
                        entry
                            .file_name()
                            .into_string()
                            .map_err(|err| {
                                tracing::warn!("Could not convert file name to string: {err:?}")
                            })
                            .ok()?
                            .strip_suffix(".json")?,
                    )
                    .map_err(|err| tracing::warn!("Could not parse UUID: {err:?}"))
                    .ok()?;

                    let asset = Asset::<T>::read(
                        std::fs::File::open(entry.path()).ok()?,
                        AssetId::from_uuid(id),
                    )
                    .map_err(|err| {
                        UiAction::Error(format!(
                            "Could not read/parse asset {}: {err:?}",
                            entry.path().display()
                        ))
                        .enqueue();
                    })
                    .ok()?;

                    return Some((id, asset));
                }

                None
            })
            .collect();

        Self(folders)
    }

    /// returns all assets in the collection
    pub fn assets(&self) -> Vec<Arc<Asset<T>>> {
        self.0.values().cloned().collect()
    }

    pub fn get(&self, id: &AssetId<T>) -> Option<&Arc<Asset<T>>> {
        self.0.get(&id.id)
    }

    /// Overwrite asset in memory cache
    pub fn set_asset(&mut self, asset: Asset<T>) {
        self.0.insert(asset.id.id, Arc::new(asset));
    }

    pub fn delete_asset(&mut self, id: AssetId<T>) {
        self.0.remove(&id.id);
    }
}
