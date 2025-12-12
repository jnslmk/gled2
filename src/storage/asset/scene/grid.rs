use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::storage::asset::scene::instance::SceneInstance;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GridLocation {
    pub col: usize,
    pub row: usize,
}

