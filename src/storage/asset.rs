use super::{collection::Collection, Action, AssetId, State, STATE};
use egui::Ui;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{fmt::Debug, fs::File, sync::Arc};

pub trait AssetTrait:
    Serialize + DeserializeOwned + Debug + Send + Sync + Clone + Default + 'static
{
    const DIR_NAME: &'static str;

    fn tree_entry_show(&self, ui: &mut Ui) {
        let _ = ui;
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
    pub name: Vec<String>,
    pub data: T,
}

impl<T: AssetTrait> Asset<T> {
    pub fn new(name: Vec<String>) -> Self {
        Self {
            id: AssetId::<T>::new(),
            name,
            data: T::default(),
        }
    }

    pub fn get(id: AssetId<T>) -> Arc<Self> {
        let state: &State = &STATE.lock();
        if let State::Opened { collections, .. } = state {
            if let Some(asset) = collections
                .get::<Collection<T>>()
                .and_then(|collection| collection.get(&id))
            {
                return asset.clone();
            }
        }

        Arc::new(Asset {
            id: id.to_owned(),
            name: Default::default(),
            data: Default::default(),
        })
    }

    pub fn all() -> Vec<Arc<Asset<T>>> {
        let state: &State = &STATE.lock();
        if let State::Opened { collections, .. } = state {
            if let Some(collection) = collections.get::<Collection<T>>() {
                return collection.assets();
            }
        }

        Default::default()
    }

    pub fn save(self) {
        log::info!("Setting asset in cache: {:?}", self.id);

        std::thread::spawn(move || {
            let state: &mut State = &mut STATE.lock();
            if let State::Opened { collections, .. } = state {
                collections
                    .entry::<Collection<T>>()
                    .or_insert_with(Default::default)
                    .set_asset(self.clone());
            }

            let uuid = self.id.id;
            if let Ok(json) = self.into_json() {
                Action::SaveAsset {
                    dir_name: T::DIR_NAME,
                    uuid,
                    json,
                }
                .send();
            }
        });
    }

    pub fn delete(&self) {
        self.id.delete();
    }

    pub fn read(file: File, id: AssetId<T>) -> Result<Arc<Self>, simd_json::Error> {
        let asset: AssetOnDisk<T> = simd_json::from_reader(file)?;

        Ok(Arc::new(Self {
            id,
            name: asset.name,
            data: asset.data,
        }))
    }

    pub fn into_json(self) -> Result<String, simd_json::Error> {
        let asset = AssetOnDisk {
            name: self.name,
            data: self.data,
        };

        simd_json::to_string_pretty(&asset)
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
