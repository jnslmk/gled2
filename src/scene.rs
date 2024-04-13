use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq, Clone)]
pub struct Scene {
    /// relative paths to effect from storage
    effect_paths: Vec<PathBuf>,
}
