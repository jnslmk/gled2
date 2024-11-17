use super::{AssetId, AssetTrait, Palette};
use crate::{
    app::Svg, group::Groups, output_routings::OutputRoutings, scene_instance::SceneInstance,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Project {
    pub a: Deck,
    pub b: Deck,
    /// -1.0 = A, 0.0 = A + B, 1.0 = B
    pub cross_fader: f32,
    pub auto_mode_active: bool,
    pub auto_mode_seconds: u64,
    pub auto_mode_max_scenes: usize,
    pub svg: Option<Svg>,
    pub output_routings: OutputRoutings,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Deck {
    pub palette: Option<AssetId<Palette>>,
    pub scene_groups: Vec<SceneGroup>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct SceneGroup {
    pub groups: Groups,
    pub scenes_instances: Vec<SceneInstance>,
}

impl AssetTrait for Project {
    const DIR_NAME: &'static str = "projects";
    const NAME: &'static str = "Project";
}
