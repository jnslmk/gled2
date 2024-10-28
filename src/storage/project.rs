use super::{
    all_projects,
    asset::{Asset, AssetTrait},
    get_project, set_project_in_cache, Action, AssetId, Palette, Scene,
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
    const DIR_NAME: &'static str = "projects";

    fn get(id: &AssetId<Project>) -> Arc<Asset<Self>> {
        get_project(id)
    }

    fn all() -> Vec<Arc<Asset<Self>>> {
        all_projects()
    }

    fn save(asset: Asset<Self>) {
        set_project_in_cache(asset.clone());

        Action::SaveProject { project: asset }.send();
    }
}
