mod folder;
mod folders;

use super::{
    action::Action, set_palette_in_cache, set_project_in_cache, set_scene_in_cache, AssetPath,
    Palette, Project, Scene,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{fmt::Debug, marker::PhantomData, sync::Arc};

pub use folders::Folders;

pub trait AssetTrait: Serialize + DeserializeOwned + Debug + Send + Sync + Clone {
    fn find(path: &AssetPath) -> Asset<Self>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "StoredAsset<T>", bound = "T: DeserializeOwned + Serialize")]
pub struct Asset<T: AssetTrait> {
    pub path: AssetPath,
    #[serde(skip_serializing)]
    pub data: Arc<T>,
}

pub struct ChangingAsset<T: AssetTrait> {
    pub path: AssetPath,
    pub data: T,
}

impl<T: AssetTrait> From<Asset<T>> for ChangingAsset<T> {
    fn from(asset: Asset<T>) -> Self {
        Self {
            path: asset.path,
            data: Arc::unwrap_or_clone(asset.data),
        }
    }
}

impl<T: AssetTrait> From<ChangingAsset<T>> for Asset<T> {
    fn from(asset: ChangingAsset<T>) -> Self {
        Self {
            path: asset.path,
            data: Arc::new(asset.data),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Asset on disk, basically just the path to the asset
pub struct StoredAsset<T> {
    pub path: AssetPath,
    #[serde(skip)]
    _phantom: std::marker::PhantomData<T>,
}

impl<T: AssetTrait> From<StoredAsset<T>> for Asset<T> {
    fn from(stored: StoredAsset<T>) -> Self {
        T::find(&stored.path)
    }
}

impl Asset<Palette> {
    pub fn save(self) {
        set_palette_in_cache(self.clone());

        Action::SavePalette { palette: self }.send();
    }
}
impl ChangingAsset<Palette> {
    pub fn save(self) {
        Asset::from(self).save();
    }
}

impl Asset<Project> {
    pub fn save(self) {
        set_project_in_cache(self.clone());

        Action::SaveProject { project: self }.send();
    }
}
impl ChangingAsset<Project> {
    pub fn save(self) {
        Asset::from(self).save();
    }
}

impl Asset<Scene> {
    pub fn save(self) {
        set_scene_in_cache(self.clone());

        Action::SaveScene { scene: self }.send();
    }
}
impl ChangingAsset<Scene> {
    pub fn save(self) {
        Asset::from(self).save();
    }
}

impl<T: AssetTrait> From<Asset<T>> for StoredAsset<T> {
    fn from(asset: Asset<T>) -> Self {
        Self {
            path: asset.path,
            _phantom: PhantomData,
        }
    }
}
