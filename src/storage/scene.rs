use super::{
    asset::{Asset, AssetTrait},
    find_scene, AssetId,
};
use crate::effect::Effect;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq, Clone)]
pub struct Scene {
    primary_effects: Vec<Effect>,
    secondary_effects: Vec<Effect>,
}

impl AssetTrait for Scene {
    fn find(id: &AssetId<Scene>) -> Arc<Asset<Self>> {
        find_scene(id)
    }
}
pub struct SceneInstance {
    path: PathBuf,
    /// cloned from the scene cache
    /// TODO: each change should be saved back to the cache and the git repo and each other scene instance with the same path must be updated
    scene: Scene,
}
