use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct AssetId<T> {
    pub id: Uuid,
    #[serde(skip)]
    _phantom: std::marker::PhantomData<T>,
}

impl<T: std::hash::Hash> std::hash::Hash for AssetId<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T> AssetId<T> {
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn disk_path(&self, root: &Path) -> PathBuf {
        root.join(format!("{}.json", self.id))
    }
}
