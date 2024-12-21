use super::deck::DeckPath;

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

    pub const DECK_A: Self = Self {
        deck_path: DeckPath::A,
        scene_instance: 0,
    };

    pub const DECK_B: Self = Self {
        deck_path: DeckPath::B,
        scene_instance: 0,
    };

    pub const DECK_C: Self = Self {
        deck_path: DeckPath::C,
        scene_instance: 0,
    };
}

impl Default for SceneInstancePath {
    fn default() -> Self {
        Self::DECK_A
    }
}
