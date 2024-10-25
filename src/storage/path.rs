use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetPath {
    pub folder: Arc<String>,
    pub file: Arc<String>,
}

impl AssetPath {
    pub fn new(folder: String, file: String) -> Self {
        Self {
            folder: Arc::new(folder),
            file: Arc::new(file),
        }
    }

    pub fn disk_path(&self, root: &Path) -> PathBuf {
        root.join(self.folder.as_str()).join(self.file.as_str())
    }
}
