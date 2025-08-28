use crate::storage::asset::project::DeckPath;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneInstancePath {
    pub deck_path: DeckPath,
    pub scene_instance: usize,
}

impl SceneInstancePath {
    pub fn new(deck_path: DeckPath, scene_instance: usize) -> Self {
        Self {
            deck_path,
            scene_instance,
        }
    }

    pub const GRID: Self = Self {
        deck_path: DeckPath::Grid,
        scene_instance: 0,
    };

    pub const QUICK: Self = Self {
        deck_path: DeckPath::Quick,
        scene_instance: 0,
    };
}

impl Default for SceneInstancePath {
    fn default() -> Self {
        Self::GRID
    }
}
