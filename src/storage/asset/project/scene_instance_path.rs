#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneInstancePath {
    pub deck_a: bool,
    pub scene_group: usize,
    pub scene_instance: usize,
}

impl SceneInstancePath {
    pub const DECK_A: Self = Self {
        deck_a: true,
        scene_group: 0,
        scene_instance: 0,
    };

    pub const DECK_B: Self = Self {
        deck_a: false,
        scene_group: 0,
        scene_instance: 0,
    };
}

impl Default for SceneInstancePath {
    fn default() -> Self {
        Self::DECK_A
    }
}
