use super::AssetId;
use egui::Ui;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{fmt::Debug, fs::File, sync::Arc};

pub trait AssetTrait:
    Serialize + DeserializeOwned + Debug + Send + Sync + Clone + PartialEq
{
    const DIR_NAME: &'static str;
    fn get(path: &AssetId<Self>) -> Arc<Asset<Self>>;
    fn all() -> Vec<Arc<Asset<Self>>>;
    fn save(asset: Asset<Self>);
    fn tree_entry_show(&self, ui: &mut Ui) {
        let _ = ui;
    }
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

    pub fn dir(&self) -> Vec<String> {
        self.name[..self.name.len() - 1].to_owned()
    }

    pub fn change_dir(&mut self, new_dir: &[String]) {
        let name = self.name.pop();
        self.name = new_dir.to_owned();
        if let Some(name) = name {
            self.name.push(name);
        }
    }
}
