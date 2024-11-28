mod animation;
mod curve;
mod output_device;
mod palette;
mod project;
mod scene;

use super::{collection::Collection, Action, AssetId, State, STATE};
use egui::Rect;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{fmt::Debug, fs::File, sync::Arc};

pub use animation::{Animation, AnimationConfig, AnimationRenderer};
pub use curve::{Curve, RangeDegrees, RangePercentage, StaticOrCurve};
pub use output_device::OutputDevice;
pub use palette::Palette;
pub use project::{Project, RenderDeactivatedScenes, SceneInstancePath};
pub use scene::Scene;

pub trait AssetTrait:
    Serialize + DeserializeOwned + Debug + Default + Send + Sync + Clone + 'static
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

    pub fn get(id: AssetId<T>) -> Option<Arc<Self>> {
        let state: &State = &STATE.lock();
        let State::Opened { collections, .. } = state else {
            return None;
        };
        collections.get::<Collection<T>>()?.get(&id).cloned()
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
