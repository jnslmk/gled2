pub mod scene_instance_path;

use super::{
    animation::Animation, output_device::routing::OutputRoutings, scene::instance::SceneInstance,
    AssetTrait,
};
use crate::midi::akai_apc40_mk2::{GRID_HEIGHT, GRID_WIDTH};
use crate::storage::asset::project::scene_instance_path::SceneInstanceUnion;
use crate::storage::asset::scene::grid::GridLocation;
use crate::{
    app::svg::Svg,
    input::{
        artnet::ArtnetConfig,
        event::{GamepadEvent, InputEvent},
    },
    pipeline::{
        group::Groups, preview::Preview
        ,
    },
    storage::{
        asset::{palette::Palette, scene::Scene},
        asset_id::AssetId,
    },
    ui::windows::channel_overwrites::ChannelOverwrites
    ,
};
use rand::seq::IndexedMutRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{
    collections::BTreeSet,
    time::Instant,
};

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
        Current(HashMap<GridLocation, SceneInstance>),
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
        Variants::Current(instances) => Ok(instances),
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
    pub fn next_empty_grid_location(&self, start: GridLocation) -> GridLocation {
        for row in 0..GRID_HEIGHT {
            let row = (start.row + row) % GRID_HEIGHT;
            for col in 0..GRID_WIDTH {
                let col = (start.col + col) % GRID_WIDTH;
                let location = GridLocation { row, col };
                if !self.scenes_instances_grid.contains_key(&location) {
                    return location;
                }
            }
        }
        start
    }

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
    pub fn location_by_location_or_quick_index(
        &mut self,
        index_or_grid: SceneInstanceUnion,
    ) -> Option<GridLocation> {
        match index_or_grid {
            SceneInstanceUnion::Grid(location) => Some(location),
            SceneInstanceUnion::Quick(quick_scene_instance_index) => Some(GridLocation {
                row: GRID_HEIGHT - 1,
                col: quick_scene_instance_index.index,
            }),
        }
    }

    pub fn all_scene_instance_locations(&self) -> impl Iterator<Item = &GridLocation> {
        self.scenes_instances_grid.keys()
    }

    pub fn scenes_instances_quick(&self) -> impl Iterator<Item = (&GridLocation, &SceneInstance)> {
        self.scenes_instances_grid
            .iter()
            .filter(|(location, _)| location.row == GRID_HEIGHT - 1)
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
    pub fn remove_scene_instance(&mut self, pos: GridLocation) -> Option<SceneInstance> {
        self.scenes_instances_grid.remove(&pos)
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

    pub fn add_scene(&mut self, pos: GridLocation, scene: AssetId<Scene>) {
        let mut scene_instance: SceneInstance = scene.into();
        scene_instance.init_states();
        self.add_scene_instance(pos, scene_instance);
    }

    pub fn add_scene_instance(&mut self, pos: GridLocation, scene_instance: SceneInstance) {
        let scene_instances = &mut self.scenes_instances_grid;
        scene_instances.insert(pos, scene_instance);
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

pub fn grid_location_from_continuous_index(index: usize, start: &GridLocation) -> GridLocation {
    let index = index + start.col + start.row * GRID_WIDTH;
    let row = index / GRID_HEIGHT;
    let col = index % GRID_WIDTH;
    GridLocation { row, col }
}