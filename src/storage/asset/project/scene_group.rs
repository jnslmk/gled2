use crate::app::Timing;
use crate::scene_instance::SceneInstance;
use crate::storage::{Asset, Palette, Scene};
use crate::{group::Groups, storage::AssetId};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wgpu::CommandEncoder;

use super::render_deactivated_scenes::RenderDeactivatedScenes;
use super::scene_instance_path::SceneInstancePath;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct SceneGroup {
    pub groups: Groups,
    pub scenes_instances: Vec<SceneInstance>,
}

impl SceneGroup {
    pub fn add_scene(&mut self, path: &mut SceneInstancePath, scene: AssetId<Scene>) {
        let mut scene_instance: SceneInstance = scene.into();
        scene_instance.init_states();
        self.scenes_instances.push(scene_instance);

        path.scene_instance = self.scenes_instances.len() - 1;
    }

    #[inline(always)]
    pub fn scene_instances(
        &mut self,
        path: SceneInstancePath,
    ) -> impl Iterator<Item = (SceneInstancePath, &mut SceneInstance)> {
        self.scenes_instances
            .iter_mut()
            .enumerate()
            .map(move |(scene_instance, si)| {
                (
                    SceneInstancePath {
                        scene_instance,
                        ..path
                    },
                    si,
                )
            })
    }

    #[inline(always)]
    pub fn scene_instance(&mut self, path: SceneInstancePath) -> Option<&mut SceneInstance> {
        self.scenes_instances.get_mut(path.scene_instance)
    }

    pub fn prepare(
        &mut self,
        path: SceneInstancePath,
        queue: &wgpu::Queue,
        timing: &Timing,
        render_deactivated_scenes: RenderDeactivatedScenes,
        palette: Option<Arc<Asset<Palette>>>,
    ) {
        let groups = self.groups.clone();
        for (path, scene_instance) in self.scene_instances(path) {
            scene_instance.prepare(
                queue,
                render_deactivated_scenes.should_render(path),
                palette.clone(),
                &groups,
                timing,
            );
        }
    }

    pub fn render(
        &mut self,
        path: SceneInstancePath,
        encoder: &mut CommandEncoder,
        blackout: bool,
        render_deactivated_scenes: RenderDeactivatedScenes,
    ) {
        for (path, scene_instance) in self.scene_instances(path) {
            scene_instance.render(
                encoder,
                blackout,
                render_deactivated_scenes.should_render(path),
            );
        }
    }
}
