use super::{
    all_scenes,
    asset::{Asset, AssetTrait},
    get_scene, AssetId,
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
    const DIR_NAME: &'static str = "scenes";

    fn get(id: &AssetId<Scene>) -> Arc<Asset<Self>> {
        get_scene(id)
    }

    fn all() -> Vec<Arc<Asset<Self>>> {
        all_scenes()
    }
}

pub struct SceneInstance {
    path: PathBuf,
    /// cloned from the scene cache
    /// TODO: each change should be saved back to the cache and the git repo and each other scene instance with the same path must be updated
    scene: Scene,
}
