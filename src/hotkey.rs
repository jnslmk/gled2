use std::collections::HashSet;

use egui::{Context, Key, Modifiers};
use gilrs::{Axis, Button, Event, Gilrs};
use log::debug;
use serde::{Deserialize, Serialize};
use strum::Display;

pub struct Gamepad {
    gilrs: Gilrs,
    events: HashSet<GamepadEvent>,
    new_events: HashSet<GamepadEvent>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Hotkey {
    Key(Key),
    GamepadEvent(GamepadEvent),
}

impl Hotkey {
    pub fn get(ctx: &Context, gamepad: &Gamepad) -> Option<Self> {
        let keys: Vec<Key> = ctx.input_mut(|i| i.keys_down.drain().collect());
        let keys_ = keys.len();
        if let Some(key) = keys.into_iter().next().filter(|_| keys_ == 1) {
            return Some(Self::Key(key));
        }

        let events = gamepad.new_events();
        if events.len() == 1 {
            events.into_iter().next().map(Self::GamepadEvent)
        } else {
            None
        }
    }

    pub fn pressed(&self, ctx: &Context, gamepad: &Gamepad) -> bool {
        match self {
            Hotkey::Key(key) => {
                !ctx.wants_keyboard_input()
                    && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, *key))
            }
            Hotkey::GamepadEvent(event) => gamepad.new_events().contains(event),
        }
    }
}

impl std::fmt::Display for Hotkey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&match self {
            Hotkey::Key(key) => format!("{key:?}"),
            Hotkey::GamepadEvent(event) => format!("{event}"),
        })
    }
}

#[derive(Serialize, Deserialize, Display, Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl Gamepad {
    pub fn new() -> Self {
        let gilrs = Gilrs::new().expect("Could not initialize gilrs");
        Self {
            gilrs,
            events: HashSet::new(),
            new_events: HashSet::new(),
        }
    }

    pub fn tick(&mut self) {
        self.new_events.clear();
        let mut still_active = HashSet::new();

        while let Some(event) = self
            .gilrs
            .next_event()
            .and_then(|Event { id, event, time }| {
                let gamepad_id: usize = id.into();

                debug!("{:?} New event from {}: {:?}", time, id, event);
                match event {
                    gilrs::EventType::ButtonChanged(button, value, _) if value > 0.5 => {
                        Some(match button {
                            Button::South => GamepadEvent::South(gamepad_id),
                            Button::East => GamepadEvent::East(gamepad_id),
                            Button::North => GamepadEvent::North(gamepad_id),
                            Button::West => GamepadEvent::West(gamepad_id),
                            Button::C => GamepadEvent::C(gamepad_id),
                            Button::Z => GamepadEvent::Z(gamepad_id),
                            Button::LeftTrigger => GamepadEvent::LeftTrigger(gamepad_id),
                            Button::LeftTrigger2 => GamepadEvent::LeftTrigger2(gamepad_id),
                            Button::RightTrigger => GamepadEvent::RightTrigger(gamepad_id),
                            Button::RightTrigger2 => GamepadEvent::RightTrigger2(gamepad_id),
                            Button::Select => GamepadEvent::Select(gamepad_id),
                            Button::Start => GamepadEvent::Start(gamepad_id),
                            Button::Mode => GamepadEvent::Mode(gamepad_id),
                            Button::LeftThumb => GamepadEvent::LeftThumb(gamepad_id),
                            Button::RightThumb => GamepadEvent::RightThumb(gamepad_id),
                            Button::DPadUp => GamepadEvent::DPadUp(gamepad_id),
                            Button::DPadDown => GamepadEvent::DPadDown(gamepad_id),
                            Button::DPadLeft => GamepadEvent::DPadLeft(gamepad_id),
                            Button::DPadRight => GamepadEvent::DPadRight(gamepad_id),
                            Button::Unknown => GamepadEvent::Unknown(gamepad_id),
                        })
                    }
                    gilrs::EventType::AxisChanged(axis, value, _) => {
                        // * 300.0 as some gamepads do not reach 1.0
                        let value = (value * 300.0).clamp(-255.0, 255.0) as i16;
                        match axis {
                            Axis::LeftStickX if value < -200 => {
                                Some(GamepadEvent::LeftStickXMin(gamepad_id))
                            }
                            Axis::LeftStickX if value > 200 => {
                                Some(GamepadEvent::LeftStickXMax(gamepad_id))
                            }
                            Axis::LeftStickY if value < -200 => {
                                Some(GamepadEvent::LeftStickYMin(gamepad_id))
                            }
                            Axis::LeftStickY if value > 200 => {
                                Some(GamepadEvent::LeftStickYMax(gamepad_id))
                            }
                            Axis::LeftZ if value < -200 => Some(GamepadEvent::LeftZMin(gamepad_id)),
                            Axis::LeftZ if value > 200 => Some(GamepadEvent::LeftZMax(gamepad_id)),
                            Axis::RightStickX if value < -200 => {
                                Some(GamepadEvent::RightStickXMin(gamepad_id))
                            }
                            Axis::RightStickX if value > 200 => {
                                Some(GamepadEvent::RightStickXMax(gamepad_id))
                            }
                            Axis::RightStickY if value < -200 => {
                                Some(GamepadEvent::RightStickYMin(gamepad_id))
                            }
                            Axis::RightStickY if value > 200 => {
                                Some(GamepadEvent::RightStickYMax(gamepad_id))
                            }
                            Axis::RightZ if value < -200 => {
                                Some(GamepadEvent::RightZMin(gamepad_id))
                            }
                            Axis::RightZ if value > 200 => {
                                Some(GamepadEvent::RightZMax(gamepad_id))
                            }
                            Axis::DPadX if value < -200 => Some(GamepadEvent::DPadXMin(gamepad_id)),
                            Axis::DPadX if value > 200 => Some(GamepadEvent::DPadXMax(gamepad_id)),
                            Axis::DPadY if value < -200 => Some(GamepadEvent::DPadYMin(gamepad_id)),
                            Axis::DPadY if value > 200 => Some(GamepadEvent::DPadYMax(gamepad_id)),
                            _ => None,
                        }
                    }
                    _ => None,
                }
            })
        {
            if self.events.insert(event) {
                self.new_events.insert(event);
            } else {
                still_active.insert(event);
            }
        }

        self.events = self
            .new_events
            .iter()
            .chain(still_active.iter())
            .copied()
            .collect();
    }

    pub fn new_events(&self) -> HashSet<GamepadEvent> {
        self.new_events.clone()
    }
}
