use crate::storage::asset::scene::grid::GridLocation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuickSceneInstanceIndex {
    pub index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneInstanceUnion {
    Selected,
    Grid(GridLocation),
    Quick(QuickSceneInstanceIndex),
}

pub fn selected_scene_instance_index() -> SceneInstanceUnion {
    SceneInstanceUnion::Selected
}

pub fn quick_scene_instance_index(index: usize) -> SceneInstanceUnion {
    SceneInstanceUnion::Quick(QuickSceneInstanceIndex { index })
}
pub fn grid_scene_instance_index(location: GridLocation) -> SceneInstanceUnion {
    SceneInstanceUnion::Grid(location)
}
