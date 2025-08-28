use crate::storage::asset::project::DeckPath;
use uuid::Uuid;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneInstancePathId {
    pub deck_path: DeckPath,
    pub id: Uuid,
}

impl SceneInstancePathId {
    pub fn new(deck_path: DeckPath, uuid: Uuid) -> Self {
        Self {
            deck_path,
            id: uuid,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneInstancePathIndex {
    pub deck_path: DeckPath,
    pub index: usize,
}

impl SceneInstancePathIndex {
    pub fn new(deck_path: DeckPath, index: usize) -> Self {
        Self { deck_path, index }
    }
}
