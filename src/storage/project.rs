use super::{
    all_projects,
    asset::{Asset, AssetTrait},
    get_project, AssetId, Palette, Scene,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq, Clone)]
pub struct Project {
    pub a: Vec<SceneGroup>,
    pub b: Vec<SceneGroup>,
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq, Clone)]
pub struct Deck {
    /// The palette is set into the deck once it is selected and can also be changed on the fly without changing the original palette
    pub palette: Palette,
    pub scene_groups: Vec<SceneGroup>,
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq, Clone)]
pub struct SceneGroup {
    pub primary_group: String,
    pub secondary_group: Option<String>,
    pub scenes: Vec<AssetId<Scene>>,
}

impl AssetTrait for Project {
    fn get(id: &AssetId<Project>) -> Arc<Asset<Self>> {
        get_project(id)
    }

    fn all() -> Vec<Arc<Asset<Self>>> {
        all_projects()
    }
}
