use super::{
    action::Action, set_palette_in_cache, set_project_in_cache, set_scene_in_cache, AssetId,
    Palette, Project, Scene,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{fmt::Debug, fs::File, sync::Arc};

pub trait AssetTrait: Serialize + DeserializeOwned + Debug + Send + Sync + Clone {
    fn find(path: &AssetId<Self>) -> Arc<Asset<Self>>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset<T: AssetTrait> {
    pub id: AssetId<T>,
    pub name: Vec<String>,
    pub data: T,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound = "T: DeserializeOwned + Serialize")]
struct AssetOnDisk<T: AssetTrait> {
    name: Vec<String>,
    #[serde(flatten)]
    data: T,
}

impl<T: AssetTrait> Asset<T> {
    pub fn read(file: File, id: AssetId<T>) -> Result<Arc<Self>, simd_json::Error> {
        let asset: AssetOnDisk<T> = simd_json::from_reader(file)?;

        Ok(Arc::new(Self {
            id,
            name: asset.name,
            data: asset.data,
        }))
    }

    pub fn write(self, file: File) -> Result<(), simd_json::Error> {
        let asset = AssetOnDisk {
            name: self.name,
            data: self.data,
        };

        simd_json::to_writer(file, &asset)
    }
}

impl Asset<Palette> {
    pub fn save(self) {
        set_palette_in_cache(self.clone());

        Action::SavePalette { palette: self }.send();
    }
}

impl Asset<Project> {
    pub fn save(self) {
        set_project_in_cache(self.clone());

        Action::SaveProject { project: self }.send();
    }
}

impl Asset<Scene> {
    pub fn save(self) {
        set_scene_in_cache(self.clone());

        Action::SaveScene { scene: self }.send();
    }
}
