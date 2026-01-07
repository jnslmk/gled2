use crate::storage::asset::project::DeckPath;
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
