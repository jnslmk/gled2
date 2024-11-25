use super::{
    render_deactivated_scenes::RenderDeactivatedScenes, scene_group::SceneGroup,
    scene_instance_path::SceneInstancePath,
};
use crate::{
    app::Timing,
    scene_instance::SceneInstance,
    storage::{Asset, AssetId, Palette},
    transition::{Transition, TransitionGoal},
};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};
use wgpu::CommandEncoder;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Deck {
    pub palette: Option<AssetId<Palette>>,
    pub scene_groups: Vec<SceneGroup>,
    pub auto_mode_active: bool,
    pub auto_mode_seconds: u64,
    pub auto_mode_max_scenes: usize,

    #[serde(skip)]
    pub auto_mode_last_change: Option<Instant>,
}

impl Deck {
    pub fn scene_groups(
        &mut self,
        path: SceneInstancePath,
    ) -> impl Iterator<Item = (SceneInstancePath, &mut SceneGroup)> {
        self.scene_groups
            .iter_mut()
            .enumerate()
            .map(move |(scene_group, sg)| {
                (
                    SceneInstancePath {
                        scene_group,
                        ..path
                    },
                    sg,
                )
            })
    }

    #[inline(always)]
    pub fn scene_group(&mut self, path: SceneInstancePath) -> Option<&mut SceneGroup> {
        self.scene_groups.get_mut(path.scene_group)
    }

    #[inline(always)]
    pub fn scene_instances(
        &mut self,
        path: SceneInstancePath,
    ) -> impl Iterator<Item = (SceneInstancePath, &mut SceneInstance)> {
        self.scene_groups(path)
            .flat_map(|(path, scene_group)| scene_group.scene_instances(path))
    }

    #[inline(always)]
    pub fn scene_instance(&mut self, path: SceneInstancePath) -> Option<&mut SceneInstance> {
        self.scene_groups
            .get_mut(path.scene_group)?
            .scene_instance(path)
    }

    pub fn prepare(
        &mut self,
        path: SceneInstancePath,
        queue: &wgpu::Queue,
        timing: &Timing,
        render_deactivated_scenes: RenderDeactivatedScenes,
        fade_duration: Duration,
        main_dimmer: f32,
    ) {
        if self.auto_mode_active {
            if self
                .auto_mode_last_change
                .get_or_insert_with(Instant::now)
                .elapsed()
                .as_secs()
                > self.auto_mode_seconds
            {
                let auto_mode_max_scenes = self.auto_mode_max_scenes;
                let mut prev = HashSet::new();
                {
                    let mut paths = self
                        .scene_instances(path)
                        .filter(|(_path, scene)| scene.active)
                        .map(|(path, _scene)| path)
                        .collect::<Vec<_>>();

                    let mut disable_count = (paths.len() + 1).saturating_sub(auto_mode_max_scenes);
                    while disable_count > 0 {
                        if let Some(path) = paths.choose_mut(&mut rand::thread_rng()).copied() {
                            if prev.insert(path) {
                                disable_count -= 1;
                                if let Some(scene) = self.scene_instance(path) {
                                    scene.set_transition(Transition::new(
                                        TransitionGoal::TurnOff,
                                        fade_duration,
                                    ));
                                }
                            }
                        }
                    }
                }

                let mut scenes = self
                    .scene_instances(path)
                    .filter(|(index, _scene)| !prev.contains(index))
                    .collect::<Vec<_>>();
                if let Some((_index, scene)) = scenes.choose_mut(&mut rand::thread_rng()) {
                    scene.set_transition(Transition::new(TransitionGoal::TurnOn, fade_duration));
                }

                self.auto_mode_last_change.take();
            }
        } else {
            self.auto_mode_last_change.take();
        }

        let palette = self.palette.and_then(Asset::get);
        for (path, scene_group) in self.scene_groups(path) {
            scene_group.prepare(
                path,
                queue,
                timing,
                render_deactivated_scenes,
                palette.clone(),
                main_dimmer,
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
        for (path, scene_group) in self.scene_groups(path) {
            scene_group.render(path, encoder, blackout, render_deactivated_scenes);
        }
    }
}
