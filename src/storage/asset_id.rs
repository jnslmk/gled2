use serde::{Deserialize, Serialize};
use std::{
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};
use uuid::Uuid;

use super::AssetTrait;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct AssetId<T> {
    pub id: Uuid,
    #[serde(skip)]
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Clone> Copy for AssetId<T> {}

impl<T> Hash for AssetId<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T: AssetTrait> AssetId<T> {
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn disk_path(&self, root: &Path) -> PathBuf {
        root.join(T::DIR_NAME).join(format!("{}.json", self.id))
    }
}
