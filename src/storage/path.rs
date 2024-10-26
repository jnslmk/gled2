use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetPath<Asset> {
    pub folder: Arc<String>,
    pub file: Arc<String>,
    _phantom: std::marker::PhantomData<Asset>,
}

impl<Asset> AssetPath<Asset> {
    pub fn new(folder: Arc<String>, file: Arc<String>) -> Self {
        Self {
            folder,
            file,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn disk_path(&self, root: &Path) -> PathBuf {
        root.join(self.folder.as_str()).join(self.file.as_str())
    }
}
