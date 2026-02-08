use crate::storage::asset::project::{grid_location_from_continuous_index, Project};
use crate::storage::asset::scene::grid::GridLocation;
use artnet_protocol::PaddedData;
use crossbeam_channel::Receiver;
use deku::prelude::*;

const ARTNET_CONTROL_SLOTS: usize = 1;
const SCENE_SPECIFIC_PARAMETERS: usize = 10;

#[derive(Debug, DekuRead, Default)]
pub struct ArtnetSceneControlState {
    pub scene_index: u8, // 0: disabled
    pub group_index: u8,
    pub opacity: u8,
    pub offset: u8,
    pub speed_multiplier: u8, // x -> 2^x; 126 = 1/2, 127 = 1, 128 = 2, ...

    pub color_mode: u8, // GledDefault = 0..=84, ColorOverride = 85..=170, PalletOverride = 171..
    pub pallet_override: u8,
    //pub color_override_primary: Color,
    //pub color_override_secondary: Color,
    
    //pub scene_specific_parameters: [u8; SCENE_SPECIFIC_PARAMETERS],
}

pub struct ExternalControlState {
    artnet_control_receiver: Receiver<PaddedData>,
}

impl ExternalControlState{
    pub fn new(artnet_control_receiver: Receiver<PaddedData>) -> Self {
        Self {
            artnet_control_receiver,
        }
    }

    pub fn process_events(&mut self, project: &mut Option<Project>) {
        loop {
            if let Ok(dmx_data) = self.artnet_control_receiver.try_recv() {
                let dmx_data = dmx_data.as_ref();
                let mut remaining = (&dmx_data[..], 0);
                let mut scene_state = ArtnetSceneControlState::default();

                for i in 0..ARTNET_CONTROL_SLOTS {
                    (remaining, scene_state) = match ArtnetSceneControlState::from_bytes(remaining) {
                        Ok(result) => result,
                        Err(_) => break,
                    };
                    if let Some(project) = project {
                        let grid_location = grid_location_from_continuous_index(i, &GridLocation { row: 0, col: 0 });
                        let scene_instance = project.scenes_instances_grid.get_mut(&grid_location);
                        if let Some(scene_instance) = scene_instance {
                            // scene_instance.scene = scene_state.scene_index; // TODO
                            // scene_instance.groups_overwrite = scene_state.group_index; // TODO
                            scene_instance.opacity.multiplier = scene_state.opacity as f32 / 255.0;
                            scene_instance.beat_progression_offset.multiplier = scene_state.speed_multiplier as f32 / 127.0;
                            scene_instance.effect_states.iter_mut().for_each(|effect_state| {
                                effect_state.speed_exponent = scene_state.speed_multiplier as i32;
                                // TODO color overwrite, scene specific parameters
                            })
                        }
                    }
                }
            }
            else{break;}
        }
    }
}