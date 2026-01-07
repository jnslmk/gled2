pub mod scene_instance_path;

use super::{
    animation::Animation, output_device::routing::OutputRoutings, scene::instance::SceneInstance,
    AssetTrait,
};
use crate::midi::akai_apc40_mk2::{GRID_HEIGHT, GRID_WIDTH};
use crate::storage::asset::project::scene_instance_path::SceneInstanceUnion;
use crate::storage::asset::scene::grid::GridLocation;
use crate::{
    app::{svg::Svg, timing::Timing},
    input::{
        artnet::ArtnetConfig,
        event::{GamepadEvent, InputEvent},
    },
    pipeline::{
        extract_output::ExtractOutput, group::Groups, output_clear::OutputClear, preview::Preview,
        preview_indices::PreviewIndices, renderer_callback::RendererCallback,
    },
    storage::{
        asset::{palette::Palette, scene::Scene, Asset},
        asset_id::AssetId,
    },
    ui::windows::channel_overwrites::ChannelOverwrites,
    wgpu_render_state,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{
    collections::BTreeSet,
    time::{Duration, Instant},
};
use wgpu::CommandEncoderDescriptor;

pub fn deserialize_scene_instances<'de, D>(
    deserializer: D,
) -> Result<HashMap<GridLocation, SceneInstance>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Variants {
        V1(Vec<SceneInstance>),
        Current(Vec<HashMap<GridLocation, SceneInstance>>),
    }

    match Variants::deserialize(deserializer)? {
        Variants::V1(instances) => Ok(instances
            .into_iter()
            .enumerate()
            .map(|(idx, instance)| {
                (
                    GridLocation {
                        row: idx / GRID_HEIGHT,
                        col: idx % GRID_WIDTH,
                    },
                    instance,
                )
            })
            .collect()),
        Variants::Current(instances) => Ok(instances.into_iter().flatten().collect()),
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct Project {
    pub palette: Option<AssetId<Palette>>,
    pub auto_mode_active: bool,
    pub auto_mode_seconds: u64,
    pub auto_mode_max_scenes: usize,
    pub groups: Groups,
    #[serde(deserialize_with = "deserialize_scene_instances")]
    pub scenes_instances_grid: HashMap<GridLocation, SceneInstance>,
    /* TODO  add serde backwards compatiblity to integrate old quick scenes into the grid?
    #[serde(deserialize_with = "deserialize_scene_instances")]
    pub scenes_instances_quick: HashMap<GridLocation, SceneInstance>,
     */
    #[serde(skip)]
    pub auto_mode_last_change: Option<Instant>,
    pub svg: Option<Svg>,
    pub channel_overwrites: ChannelOverwrites,
    pub output_routings: OutputRoutings,
    pub artnet_config: ArtnetConfig,
    pub tap_input_events: BTreeSet<InputEvent>,
    pub blackout_input_events: BTreeSet<InputEvent>,
    pub blackout_hold_input_events: BTreeSet<InputEvent>,
    pub half_input_events: BTreeSet<InputEvent>,
    pub double_input_events: BTreeSet<InputEvent>,
    pub main_dimmer: f32,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            palette: None,
            auto_mode_active: false,
            auto_mode_seconds: 10,
            auto_mode_max_scenes: 2,
            groups: Groups::default(),
            scenes_instances_grid: HashMap::new(),
            auto_mode_last_change: None,
            svg: Default::default(),
            channel_overwrites: Default::default(),
            output_routings: Default::default(),
            artnet_config: Default::default(),
            tap_input_events: std::iter::once(InputEvent::Key(egui::Key::T))
                .chain(std::iter::once(InputEvent::Gamepad(GamepadEvent::Mode(0))))
                .collect(),
            blackout_input_events: std::iter::once(InputEvent::Key(egui::Key::B)).collect(),
            blackout_hold_input_events: std::iter::once(InputEvent::Key(egui::Key::N))
                .chain(std::iter::once(InputEvent::Gamepad(GamepadEvent::Start(0))))
                .collect(),
            half_input_events: std::iter::once(InputEvent::Key(egui::Key::Minus)).collect(),
            double_input_events: std::iter::once(InputEvent::Key(egui::Key::Plus)).collect(),
            main_dimmer: 1.0,
        }
    }
}

impl Project {
    pub fn get_scenes_instance(&mut self, pos: &GridLocation) -> Option<&mut SceneInstance> {
        self.scenes_instances_grid.get_mut(pos)
    }
    pub fn scenes_instances_grid_len(&self) -> usize {
        self.scenes_instances_grid.len()
    }

    pub fn scene_instance_by_location_or_quick_index(
        &mut self,
        index_or_grid: SceneInstanceUnion,
    ) -> Option<&mut SceneInstance> {
        let pos = &self.location_by_location_or_quick_index(index_or_grid)?;
        self.get_scenes_instance(pos)
    }
    pub fn location_by_location_or_quick_index(&mut self, index_or_grid: SceneInstanceUnion) -> Option<GridLocation> {
        match index_or_grid {
            SceneInstanceUnion::Grid(location) => Some(location),
            SceneInstanceUnion::Quick(quick_scene_instance_index) =>
                    Some(GridLocation { row: GRID_HEIGHT-1, col: quick_scene_instance_index.index})
        }
    }

    pub fn all_scene_instance_locations(&self) -> impl Iterator<Item = &GridLocation> {
        self.scenes_instances_grid.keys()
    }

    pub fn scenes_instances_quick(&self) -> impl Iterator<Item = (&GridLocation, &SceneInstance)> {
        self.scenes_instances_grid.iter()
            .filter(|(location, _)| {location.row == GRID_HEIGHT-1})
    }

    pub fn reload_shader_code(&mut self, animation: AssetId<Animation>) {
        self.scenes_instances_grid
            .values_mut()
            .for_each(|scene_instance| {
                scene_instance.reload_shader_code(animation);
            });
    }

    pub fn send_positions(&mut self) {
        self.scenes_instances_grid
            .values_mut()
            .for_each(|scene_instance| {
                scene_instance.send_positions();
            });
    }

    pub fn init_gpu(&mut self) {
        self.scenes_instances_grid
            .values_mut()
            .for_each(|scene_instance| {
                scene_instance.init_states();
            });
        self.set_buffers();
    }

    pub fn set_buffers(&mut self) {
        self.scenes_instances_grid
            .values_mut()
            .for_each(|scene_instance| {
                scene_instance.set_output_mix_buffers();
            });
        Preview::set_buffers();
    }

    /// Remove scene instance at path and update path to the next scene instance
    pub fn remove_scene_instance(
        &mut self,
        pos: GridLocation,
    ) -> Option<SceneInstance> {
        let scene_instance =  self.scenes_instances_grid.remove(&pos);
        scene_instance
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        timing: &Timing,
        blackout: bool,
        always_render: bool,
        fade_duration: Duration,
    ) {
        let wgpu_render_state = wgpu_render_state();
        let device = wgpu_render_state.device;
        let queue = &wgpu_render_state.queue;
        // TODO @bratorange
        //if self.auto_mode_active {
        //    if self
        //        .auto_mode_last_change
        //        .get_or_insert_with(Instant::now)
        //        .elapsed()
        //        .as_secs()
        //        > self.auto_mode_seconds
        //    {
        //        let auto_mode_max_scenes = self.auto_mode_max_scenes;
        //        let mut prev = HashSet::new();
        //        {
        //            let mut indices = self
        //                .scenes_instances_grid
        //                .iter()
        //                .filter(|(_index, scene)| scene.active)
        //                .map(|(location, _scene)| location)
        //                .collect::<Vec<_>>();
        //
        //            let mut disable_count =
        //                (indices.len() + 1).saturating_sub(auto_mode_max_scenes);
        //            while disable_count > 0 {
        //                if let Some(location) = indices.choose_mut(&mut rand::rng()).copied()
        //                    && prev.insert(location)
        //                {
        //                    disable_count -= 1;
        //                    if let Some(scene) = self.scenes_instances_grid.get_mut(location) {
        //                        scene.set_transition(Transition::new(
        //                            TransitionGoal::TurnOff,
        //                            fade_duration,
        //                        ));
        //                    }
        //                }
        //            }
        //        }
        //
        //        let mut scenes = self
        //            .scenes_instances_grid
        //            .iter_mut()
        //            .filter(|(location, _scene)| !prev.contains(location))
        //            .collect::<Vec<_>>();
        //        if let Some((_index, scene)) = scenes.choose_mut(&mut rand::rng()) {
        //            scene.set_transition(Transition::new(TransitionGoal::TurnOn, fade_duration));
        //        }
        //
        //        self.auto_mode_last_change.take();
        //    }
        //} else {
        self.auto_mode_last_change.take();
        //}

        let palette = self.palette.and_then(Asset::get);
        let deck_groups = self.groups.clone();
        let main_dimmer = self.main_dimmer;
        for scene_instance in self.scenes_instances_grid.values_mut() {
            scene_instance.prepare(
                queue,
                always_render,
                palette.clone(),
                &deck_groups,
                timing,
                main_dimmer,
            );
        }

        PreviewIndices::get().prepare(queue);

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render animations"),
        });

        OutputClear::get().run(&mut encoder);

        for scene_instance in self.scenes_instances_grid.values_mut() {
            scene_instance.render(&mut encoder, blackout, always_render);
        }

        ExtractOutput::get().run(&mut encoder);
        PreviewIndices::get().run(&mut encoder);
        Preview::run(&mut encoder);

        RendererCallback::add(encoder.finish());
    }

    pub fn tap_input_is_new(&self) -> bool {
        self.tap_input_events.iter().any(|event| event.is_new())
    }

    pub fn blackout_input_is_new(&self) -> bool {
        self.blackout_input_events
            .iter()
            .any(|event| event.is_new())
    }

    pub fn blackout_hold_input_is_live(&self) -> bool {
        self.blackout_hold_input_events
            .iter()
            .any(|event| event.is_live())
    }

    pub fn half_input_is_new(&self) -> bool {
        self.half_input_events.iter().any(|event| event.is_new())
    }

    pub fn double_input_is_new(&self) -> bool {
        self.double_input_events.iter().any(|event| event.is_new())
    }

    pub fn add_scene(&mut self, pos: &GridLocation, scene: AssetId<Scene>) {
        let mut scene_instance: SceneInstance = scene.into();
        scene_instance.init_states();
        self.add_scene_instance(pos, scene_instance);
    }

    pub fn add_scene_instance(&mut self, pos: &GridLocation, scene_instance: SceneInstance) {
        let scene_instances = &mut self.scenes_instances_grid;
        scene_instances.insert(*pos, scene_instance);
    }

    pub fn remove_nonexistant_groups(&mut self) {
        self.groups.remove_nonexistant_groups();
        for scene_instance in self.scenes_instances_grid.values_mut() {
            scene_instance.remove_nonexistant_groups();
        }
    }
}

impl AssetTrait for Project {
    const DIR_NAME: &'static str = "projects";
    const NAME: &'static str = "Project";
    const SHOW_NAME_IF_SELECTED: bool = true;
}
