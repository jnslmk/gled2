use crate::storage::asset::scene::grid::GridLocation;

#[derive(Debug)]
pub struct QuickSceneInstanceIndex {
    pub index: usize,
}

#[derive(Debug)]
pub enum SceneInstanceUnion {
    Grid(GridLocation),
    Quick(QuickSceneInstanceIndex),
}

pub fn quick_scene_instance_index(index: usize) -> SceneInstanceUnion {
    SceneInstanceUnion::Quick(QuickSceneInstanceIndex {index} )
}
pub fn grid_scene_instance_index(location: GridLocation) -> SceneInstanceUnion {
    SceneInstanceUnion::Grid(location)
}