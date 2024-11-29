use super::deck::DeckPath;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneInstancePath {
    pub deck_path: DeckPath,
    pub scene_group: usize,
    pub scene_instance: usize,
}

impl SceneInstancePath {
    pub const DECK_A: Self = Self {
        deck_path: DeckPath::A,
        scene_group: 0,
        scene_instance: 0,
    };

    pub const DECK_B: Self = Self {
        deck_path: DeckPath::B,
        scene_group: 0,
        scene_instance: 0,
    };

    pub const DECK_C: Self = Self {
        deck_path: DeckPath::C,
        scene_group: 0,
        scene_instance: 0,
    };
}

impl Default for SceneInstancePath {
    fn default() -> Self {
        Self::DECK_A
    }
}
