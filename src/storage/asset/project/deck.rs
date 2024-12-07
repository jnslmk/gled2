use super::{
    render_deactivated_scenes::RenderDeactivatedScenes, scene_instance_path::SceneInstancePath,
};
use crate::{
    app::Timing,
    group::{Group, Groups},
    scene_instance::SceneInstance,
    storage::{Asset, AssetId, Palette, Scene},
    transition::{Transition, TransitionGoal},
};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    time::{Duration, Instant},
};
use wgpu::CommandEncoder;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeckPath {
    #[default]
    A,
    B,
    C,
}

impl DeckPath {
    pub fn name(&self) -> &str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "Common Scenes",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Deck {
    pub palette: Option<AssetId<Palette>>,
    pub auto_mode_active: bool,
    pub auto_mode_seconds: u64,
    pub auto_mode_max_scenes: usize,
    #[serde(deserialize_with = "deserialize_groups")]
    pub groups: Groups,
    pub scenes_instances: Vec<SceneInstance>,

    #[serde(skip)]
    pub auto_mode_last_change: Option<Instant>,
}

impl Deck {
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
        let deck_groups = self.groups.clone();
        for (path, scene_instance) in self.scene_instances(path) {
            scene_instance.prepare(
                queue,
                render_deactivated_scenes.should_render(path),
                palette.clone(),
                &deck_groups,
                timing,
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
        for (path, scene_instance) in self.scene_instances(path) {
            scene_instance.render(
                encoder,
                blackout,
                render_deactivated_scenes.should_render(path),
            );
        }
    }

    pub fn remove_nonexistant_groups(&mut self) {
        self.groups.remove_nonexistant_groups();
        for scene_instance in &mut self.scenes_instances {
            scene_instance.remove_nonexistant_groups();
        }
    }
}

fn deserialize_groups<'de, D>(deserializer: D) -> Result<Groups, D::Error>
where
    D: serde::Deserializer<'de>,
{
    BTreeMap::<String, Group>::deserialize(deserializer).map(|map| {
        Groups::new(
            map.into_iter()
                .filter_map(|(index, group)| index.parse().ok().map(|index| (index, group)))
                .collect(),
        )
    })
}
