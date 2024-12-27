use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::state::MidiState;
use crate::{
    storage::asset::project::{deck::DeckPath, scene_instance_path::SceneInstancePath},
    ui::action::UiAction,
};
use crossbeam_channel::Receiver;
use midir::MidiOutputConnection;

pub fn handle_input(_stamp: u64, message: &[u8]) {
    if message.len() != 3 {
        return;
    }

    let status = message[0];
    let data1 = message[1];
    let data2 = message[2];
    match (status, data1, data2) {
        (144, 0..40, 127) => {
            let path = match data1 {
                0..4 => SceneInstancePath::new(DeckPath::A, data1 as usize + 16),
                4..8 => SceneInstancePath::new(DeckPath::B, data1 as usize + 12),
                8..12 => SceneInstancePath::new(DeckPath::A, data1 as usize + 4),
                12..16 => SceneInstancePath::new(DeckPath::B, data1 as usize),
                16..20 => SceneInstancePath::new(DeckPath::A, data1 as usize - 8),
                20..24 => SceneInstancePath::new(DeckPath::B, data1 as usize - 12),
                24..28 => SceneInstancePath::new(DeckPath::A, data1 as usize - 20),
                28..32 => SceneInstancePath::new(DeckPath::B, data1 as usize - 24),
                32..36 => SceneInstancePath::new(DeckPath::A, data1 as usize - 32),
                36..40 => SceneInstancePath::new(DeckPath::B, data1 as usize - 36),
                _ => unreachable!(),
            };
            UiAction::ToggleSceneActive(path)
        }
        (128..136, 48 | 52, _) => UiAction::SetSceneActive(
            SceneInstancePath::new(DeckPath::C, status as usize - 128),
            false,
        ),
        (144..152, 48 | 52, 127) => UiAction::SetSceneActive(
            SceneInstancePath::new(DeckPath::C, status as usize - 144),
            true,
        ),
        (144, 91, 127) => UiAction::SetBlackout(false),
        (128, 91, 127) => UiAction::SetBlackout(true),
        (144, 82..=85 | 99, 127) => UiAction::Tap,
        (176, 13, value) => {
            let delta = if value > 64 {
                value as f32 - 128.0
            } else {
                value as f32
            };
            UiAction::SpeedAdd(delta)
        }
        (176, 14, value) => UiAction::SetMainDimmer(f32::from(value) / 127.0),
        (176..184, 7, value) => UiAction::SetSceneOpacity(
            SceneInstancePath::new(DeckPath::C, status as usize - 176),
            f32::from(value) / 127.0,
        ),
        (144, 100 | 101, 127) => UiAction::SpeedMultiply(match data1 {
            100 => 0.5,
            101 => 2.0,
            _ => unreachable!(),
        }),
        (176, 15, value) => UiAction::SetCrossFader(f32::from(value) / 127.0),
        _ => {
            return;
        }
    }
    .enqueue();
}

pub fn send_output(state_receiver: Receiver<MidiState>, mut connection: MidiOutputConnection) {
    // Reset all lights
    for on in [true, false] {
        for j in 0x90..0x99 {
            for i in 0..127 {
                //println!("Setting {i}");
                connection.send(&[j, i, if on { 30 } else { 0 }, 127]).ok();
                //std::thread::sleep(Duration::from_secs(1));
                std::thread::sleep(Duration::from_micros(200));
            }
        }
    }

    for state in state_receiver {
        connection
            .send(&[0x90, 91, if state.blackout { 0 } else { 127 }, 127])
            .ok();

        for flank in 0..4 {
            connection
                .send(&[
                    0x90,
                    82 + flank,
                    match (state.beat_flank == flank, state.blackout) {
                        (true, false) => 30,
                        (_, true)
                            if SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .map(|d| d.as_millis() / 200 % 2 == 0)
                                .unwrap_or_default() =>
                        {
                            120
                        }
                        _ => 0,
                    },
                    127,
                ])
                .ok();
        }

        for path in { 0..20 }
            .map(|index| SceneInstancePath::new(DeckPath::A, index))
            .chain({ 0..20 }.map(|index| SceneInstancePath::new(DeckPath::B, index)))
            .chain({ 0..8 }.map(|index| SceneInstancePath::new(DeckPath::C, index)))
        {
            let active = state.active_scenes.contains(&path);
            let value = match (path.deck_path, active) {
                (DeckPath::A, true) => 30,
                (DeckPath::B, true) => 50,
                (DeckPath::C, true) => 30,
                _ => 0,
            };

            let message = match (path.deck_path, path.scene_instance) {
                (DeckPath::A, 0..4) => [0x90, path.scene_instance as u8 + 32, value, 127],
                (DeckPath::B, 0..4) => [0x90, path.scene_instance as u8 + 36, value, 127],
                (DeckPath::A, 4..8) => [0x90, path.scene_instance as u8 + 20, value, 127],
                (DeckPath::B, 4..8) => [0x90, path.scene_instance as u8 + 24, value, 127],
                (DeckPath::A, 8..12) => [0x90, path.scene_instance as u8 + 8, value, 127],
                (DeckPath::B, 8..12) => [0x90, path.scene_instance as u8 + 12, value, 127],
                (DeckPath::A, 12..16) => [0x90, path.scene_instance as u8 + 4, value, 127],
                (DeckPath::B, 12..16) => [0x90, path.scene_instance as u8, value, 127],
                (DeckPath::A, 16..20) => [0x90, path.scene_instance as u8 - 16, value, 127],
                (DeckPath::B, 16..20) => [0x90, path.scene_instance as u8 - 12, value, 127],
                (DeckPath::C, _) => [0x90 + path.scene_instance as u8, 48, value, 127],
                _ => unreachable!(),
            };
            connection.send(&message).ok();
        }
    }
}
