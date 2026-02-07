use crate::storage::asset::palette::Color;
use crate::storage::asset::scene::instance::SceneInstance;
use crate::storage::asset_id::AssetId;
use artnet_protocol::PaddedData;
use crossbeam_channel::Receiver;
use std::array;
use zerocopy::*;

const ARTNET_CONTROL_SLOTS: usize = 10;
const SCENE_SPECIFIC_PARAMETERS: usize = 10;

#[derive(FromBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct ArtnetSceneControlState {
    pub scene_index: u8, // 0: disabled
    pub group_index: u8,
    pub opacity: u8,
    pub offset: u8,
    pub speed_multiplier: u8, // x -> 2^x; 126 = 1/2, 127 = 1, 128 = 2, ...

    pub color_mode: u8, // GledDefault = 0..=84, ColorOverride = 85..=170, PalletOverride = 171..
    pub pallet_override: u8,
    pub color_override_primary: Color,
    pub color_override_secondary: Color,
    
    pub scene_specific_parameters: [u8; SCENE_SPECIFIC_PARAMETERS],
}

const FIXTURE_LENGTH: usize =
    9 // scene_index, group_index, opacity, offset, speed_multiplier, color_mode, pallet_override, ColorMode, PaletteOverride
    + 6 // color_override
    + SCENE_SPECIFIC_PARAMETERS;

pub struct ExternalControlState {
    artnet_control_receiver: Receiver<PaddedData>,
    pub scene_slots: [SceneInstance; ARTNET_CONTROL_SLOTS]
}

impl ExternalControlState{
    pub fn new(artnet_control_receiver: Receiver<PaddedData>) -> Self {
        Self {
            artnet_control_receiver,
            scene_slots: array::from_fn(|_| SceneInstance::from(AssetId::default())),
        }
    }

    pub fn process_events(&mut self) {
        loop {
            if let Ok(dmx_data) = self.artnet_control_receiver.try_recv() {
                let dmx_data = dmx_data.as_ref();
                dmx_data.chunks(FIXTURE_LENGTH).enumerate().for_each(|(slot, chunk)| {
                    let scene_state = ArtnetSceneControlState::ref_from_bytes(&chunk[..])
                        .expect("Could not parse scene control state");

                    let scene_instance = &mut self.scene_slots[slot];
                    // scene_instance.scene = scene_state.scene_index; // TODO
                    // scene_instance.groups_overwrite = scene_state.group_index; // TODO
                    scene_instance.input_dimmer = scene_state.opacity as f32 / 255.0;
                    scene_instance.beat_progression_offset.multiplier = scene_state.speed_multiplier as f32 / 127.0;
                    scene_instance.effect_states.iter_mut().for_each(|effect_state| {
                        effect_state.speed_exponent = scene_state.speed_multiplier as i32;
                        // TODO color overwrite, scene specific parameters
                    })
                    
                });
            }
            else{break;}
        }
    }
}

impl ExternalControlState{
    pub fn render(self, encoder: &mut wgpu::CommandEncoder) { // TODO

    }
}