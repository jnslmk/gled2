use super::scene_instance_path::SceneInstancePath;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderDeactivatedScenes {
    Always,
    Some(SceneInstancePath, SceneInstancePath),
}

impl RenderDeactivatedScenes {
    pub fn should_render(&self, path: SceneInstancePath) -> bool {
        match self {
            RenderDeactivatedScenes::Always => true,
            RenderDeactivatedScenes::Some(selected, hovered) => {
                *selected == path || *hovered == path
            }
        }
    }
}
