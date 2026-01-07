use uuid::Uuid;
use crate::storage::asset::scene::grid::GridLocation;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneInstancePathId {
    pub id: Uuid,
}

impl SceneInstancePathId {
    pub fn new(uuid: Uuid) -> Self {
        Self {
            id: uuid,
        }
    }
}


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