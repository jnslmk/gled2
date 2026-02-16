use crate::pipeline::preview::Preview;
use crate::storage::asset::Asset;
use crate::storage::asset::project::{Project, grid_location_from_continuous_index};
use crate::storage::asset::scene::Scene;
use crate::storage::asset::scene::grid::GridLocation;
use crate::storage::asset::scene::instance::SceneInstance;
use crate::storage::asset_id::AssetId;
use artnet_protocol::PaddedData;
use crossbeam_channel::Receiver;
use deku::prelude::*;
use serde::{Deserialize, Serialize};

const ARTNET_CONTROL_SLOTS: usize = 10;
const SCENE_SPECIFIC_PARAMETERS: usize = 10;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct ArtnetControlConfig{
    pub active: bool,
    pub universe: u16,
}

impl Default for ArtnetControlConfig {
    fn default() -> Self {
        Self { active: false, universe: 1337 }
    }
}

#[derive(Debug, DekuRead, Default, Copy, Clone)]
pub struct ArtnetSceneControlState {
    pub scene_index: u8, // 0: disabled
    pub group_index: u8,
    pub opacity: u8,
    pub offset: u8,
    pub speed_multiplier: u8, // x -> 2^x; 126 = 1/2, 127 = 1, 128 = 2, ...

    pub color_mode: u8, // GledDefault = 0..=84, ColorOverride = 85..=170, PalletOverride = 171..
    pub pallet_override: u8,
    //pub color_override_primary: Color,
    // pub color_override_secondary: Color,
    pub dummy: [u8; 6],
    pub scene_specific_parameters: [u8; SCENE_SPECIFIC_PARAMETERS],
}

pub struct ExternalControlState {
    prev_values: [ArtnetSceneControlState; ARTNET_CONTROL_SLOTS],
    artnet_control_receiver: Receiver<Vec<u8>>,
}

impl ExternalControlState {
    pub fn new(artnet_control_receiver: Receiver<Vec<u8>>) -> Self {
        Self {
            prev_values: [ArtnetSceneControlState::default(); ARTNET_CONTROL_SLOTS],
            artnet_control_receiver,
        }
    }

    pub fn process_events(&mut self, project: &mut Option<Project>) {
        loop {
            if let Ok(dmx_data) = self.artnet_control_receiver.try_recv() {
                let dmx_data = dmx_data.as_ref();

                self.process_dmx(project, &dmx_data);
            } else {
                break;
            }
        }
    }

    fn process_dmx(&mut self, project: &mut Option<Project>, dmx_data: &&Vec<u8>) {
        let mut remaining = (&dmx_data[..], 0);
        let mut scene_state: ArtnetSceneControlState;

        for i in 0..ARTNET_CONTROL_SLOTS {
            (remaining, scene_state) = match ArtnetSceneControlState::from_bytes(remaining) {
                Ok(result) => result,
                Err(_) => break,
            };
            if let Some(project) = project {
                let grid_location = location(i);
                let scene_instance = project.scenes_instances_grid.get_mut(&grid_location);

                let asset_id = Asset::get_asset_from_index(scene_state.scene_index as usize)
                    .map(|asset| asset.id);
                let scene_data = match (asset_id, scene_instance) {
                    (None, _) => {
                        project.scenes_instances_grid.remove(&grid_location);
                        None
                    }
                    (Some(asset_id), None) => {
                        project.add_scene_instance(grid_location, SceneInstance::from(asset_id));
                        project
                            .scenes_instances_grid
                            .get_mut(&grid_location)
                            .map(|scene_instance| (asset_id, scene_instance))
                    }
                    (Some(asset_id), Some(scene_instance)) => Some((asset_id, scene_instance)),
                };
                if let Some((asset_id, scene_instance)) = scene_data {
                    // check if the scene index has changed
                    let prev_state = &self.prev_values[i];
                    update_scene_instance(&mut scene_state, asset_id, scene_instance, prev_state);
                }
            } else {
                log::warn!("Received Artnet DMX data, but no project is loaded");
            }
            self.prev_values[i] = scene_state;
        }
    }
}

fn update_scene_instance(
    scene_state: &mut ArtnetSceneControlState,
    asset_id: AssetId<Scene>,
    scene_instance: &mut SceneInstance,
    prev_state: &ArtnetSceneControlState,
) {
    if prev_state.scene_index != scene_state.scene_index {
        scene_instance.scene = asset_id;
        scene_instance.init_states();
        scene_instance.set_output_mix_buffers();
        Preview::set_buffers();
    }
    scene_instance.active = scene_state.opacity != 0;
    scene_instance.opacity.multiplier = scene_state.opacity as f32 / 255.0;
    scene_instance.beat_progression_offset.multiplier = scene_state.speed_multiplier as f32 / 127.0;
    scene_instance
        .effect_states
        .iter_mut()
        .for_each(|effect_state| {
            effect_state.speed_exponent = scene_state.speed_multiplier as i32;
            // TODO color overwrite, scene specific parameters
        })
}

fn location(i: usize) -> GridLocation {
    grid_location_from_continuous_index(i, &GridLocation { row: 0, col: 0 })
}
