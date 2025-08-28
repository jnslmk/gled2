use super::Input;
use egui::Key;
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum InputEvent {
    Key(Key),
    Gamepad(GamepadEvent),
    Artnet(u8),
}

impl InputEvent {
    pub fn get() -> Option<Self> {
        Input::first_new_event()
    }

    pub fn is_new(&self) -> bool {
        Input::is_event_new(self)
    }

    pub fn is_live(&self) -> bool {
        Input::event_live_value(self).is_some()
    }

    /// get dimmer value
    pub fn dimmer(&self) -> f32 {
        Input::event_live_value(self)
            .map(|value| f32::from(value) / 255.0)
            .unwrap_or(0.0)
    }
}

impl std::fmt::Display for InputEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&match self {
            InputEvent::Key(key) => format!("Key: {key:?}"),
            InputEvent::Gamepad(event) => format!("Gamepad: {event}"),
            InputEvent::Artnet(channel) => format!("Artnet: {channel}"),
        })
    }
}

#[derive(
    Serialize, Deserialize, Display, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
pub enum GamepadEvent {
    South(usize),
    East(usize),
    North(usize),
    West(usize),
    C(usize),
    Z(usize),
    LeftTrigger(usize),
    LeftTrigger2(usize),
    RightTrigger(usize),
    RightTrigger2(usize),
    Select(usize),
    Start(usize),
    Mode(usize),
    LeftThumb(usize),
    RightThumb(usize),
    DPadUp(usize),
    DPadDown(usize),
    DPadLeft(usize),
    DPadRight(usize),
    Unknown(usize),
    LeftStickXMin(usize),
    LeftStickXMax(usize),
    LeftStickYMin(usize),
    LeftStickYMax(usize),
    LeftZMin(usize),
    LeftZMax(usize),
    RightStickXMin(usize),
    RightStickXMax(usize),
    RightStickYMin(usize),
    RightStickYMax(usize),
    RightZMin(usize),
    RightZMax(usize),
    DPadXMin(usize),
    DPadXMax(usize),
    DPadYMin(usize),
    DPadYMax(usize),
}
