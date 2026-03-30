pub mod animation;
pub mod curve;
pub mod midi_controller;
pub mod output_device;
pub mod palette;
pub mod project;
pub mod scene;

use super::{AssetId, StorageAction};
use crate::storage::collections::Collections;
use egui::Rect;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{fmt::Debug, fs::File, sync::Arc};

pub trait AssetTrait:
    Serialize + DeserializeOwned + Debug + Default + Send + Sync + Clone + 'static + PartialEq
{
    const DIR_NAME: &'static str;
    const NAME: &'static str;
    const SHOW_NAME_IF_SELECTED: bool;

    fn show(&self, ui: &mut egui::Ui, rect: Rect) {
        let _ = ui;
        let _ = rect;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound = "T: DeserializeOwned + Serialize")]
struct AssetOnDisk<T: AssetTrait> {
    name: Vec<String>,
    #[serde(flatten)]
    data: T,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset<T: AssetTrait> {
    pub id: AssetId<T>,
    pub path: Vec<String>,
    pub data: T,
}

impl<T: AssetTrait> Default for Asset<T> {
    fn default() -> Self {
        Self::new(vec![format!("New {}", T::NAME)])
    }
}

impl<T: AssetTrait> Asset<T> {
    pub fn new(path: Vec<String>) -> Self {
        Self {
            id: AssetId::<T>::new(),
            path,
            data: T::default(),
        }
    }

    /// Create a copy of an asset with a new uuid
    pub fn copy(&self) -> Self {
        Self {
            id: AssetId::<T>::new(),
            path: self.path.clone(),
            data: self.data.clone(),
        }
    }

    pub fn get(id: AssetId<T>, collections: &Collections) -> Option<Arc<Self>> {
        collections.get::<T>()?.get(&id).cloned()
    }

    pub fn get_asset_from_index(index: usize, collections: &Collections) -> Option<Arc<Self>> {
        match index {
            0 => None,
            index => Self::all(collections).get(index - 1).cloned(),
        }
    }

    pub fn all(collections: &Collections) -> Vec<Arc<Asset<T>>> {
        let Some(collections) = collections.get::<T>() else {
            return vec![];
        };

        collections.assets()
    }

    pub fn save(self, collections: &mut Collections) {
        log::info!("Setting asset in cache: {:?}", self.id);
        collections.get_mut::<T>().set_asset(self.clone());

        log::info!("Saving asset: {:?}", self.id);
        let uuid = self.id.id;
        match self.into_json() {
            Ok(json) => {
                StorageAction::SaveAsset {
                    dir_name: T::DIR_NAME,
                    uuid,
                    json,
                }
                .enqueue();
            }
            Err(err) => log::error!("Could not serialize asset {uuid}: {err}"),
        }
    }

    pub fn delete(&self, collections: &mut Collections) {
        self.id.delete(collections);
    }

    pub fn read(file: File, id: AssetId<T>) -> Result<Arc<Self>, serde_json::Error> {
        let asset: AssetOnDisk<T> = serde_json::from_reader(file)?;

        Ok(Arc::new(Self {
            id,
            path: asset.name,
            data: asset.data,
        }))
    }

    pub fn into_json(self) -> Result<String, serde_json::Error> {
        let asset = AssetOnDisk {
            name: self.path,
            data: self.data,
        };

        serde_json::to_string_pretty(&asset)
    }

    pub fn dir(&self) -> Vec<String> {
        self.path[..self.path.len() - 1].to_owned()
    }

    pub fn name(&self) -> &str {
        self.path
            .last()
            .map(|name| name.as_str())
            .unwrap_or("No name")
    }

    pub fn change_dir(&mut self, new_dir: &[String]) {
        let name = self.path.pop();
        self.path = new_dir.to_owned();
        if let Some(name) = name {
            self.path.push(name);
        }
    }
}
