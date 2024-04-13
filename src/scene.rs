use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub struct Scene {
    /// relative paths to effect from storage
    effect_paths: Vec<PathBuf>,
}
