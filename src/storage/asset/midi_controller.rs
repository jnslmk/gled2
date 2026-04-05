use super::AssetTrait;
use serde::{Deserialize, Serialize};

use crate::storage::asset::scene::color::SceneInstanceColor;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[derive(Default)]
pub struct MidiController {
    pub mapping: MidiControllerMapping,
}


impl AssetTrait for MidiController {
    const DIR_NAME: &'static str = "midi_controllers";
    const NAME: &'static str = "MIDI Controller";
    const SHOW_NAME_IF_SELECTED: bool = true;
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Default)]
pub struct MidiControllerMapping {
    pub input_bindings: Vec<MidiInputBinding>,
    pub output_bindings: Vec<MidiOutputBinding>,
    pub color_mappings: Vec<MidiNamedSceneColorMapping>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct MidiInputBinding {
    pub name: String,
    pub trigger: MidiTrigger,
    pub action: MidiInputAction,
}

impl Default for MidiInputBinding {
    fn default() -> Self {
        Self {
            name: "Input".to_owned(),
            trigger: MidiTrigger::default(),
            action: MidiInputAction::Tap,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct MidiTrigger {
    pub status: u8,
    pub data1: u8,
    pub match_data1: bool,
}

impl Default for MidiTrigger {
    fn default() -> Self {
        Self {
            status: 0,
            data1: 0,
            match_data1: true,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum MidiInputAction {
    Tap,
    SetMainDimmer,
    SetBlackout,
    SetSpeedAdd,
    SetSpeedMultiply,
    SelectScene {
        target: MidiSceneTarget,
    },
    ToggleSceneActive {
        target: MidiSceneTarget,
    },
    SetSceneActive {
        target: MidiSceneTarget,
    },
    SetSceneOpacity {
        target: MidiSceneTarget,
    },
    SetSceneInputDimmer {
        target: MidiSceneTarget,
    },
    SetSceneBeatOffset {
        target: MidiSceneTarget,
    },
    SetSceneIgnoreMainDimmer {
        target: MidiSceneTarget,
    },
    SetSceneSetOffsetOnFlash {
        target: MidiSceneTarget,
    },
    SetSceneEffectSettingF32 {
        target: MidiSceneTarget,
        effect_index: u8,
        setting_index: u8,
    },
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Default)]
pub enum MidiSceneTarget {
    #[default]
    Selected,
    Quick {
        index: u8,
    },
    Grid {
        row: u8,
        col: u8,
    },
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct MidiOutputBinding {
    pub name: String,
    pub kind: MidiOutputBindingKind,
}

impl Default for MidiOutputBinding {
    fn default() -> Self {
        Self {
            name: "Output".to_owned(),
            kind: MidiOutputBindingKind::Value(MidiValueOutput::default()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum MidiOutputBindingKind {
    Value(MidiValueOutput),
    SceneColorValue(MidiSceneColorValueOutput),
}

impl Default for MidiOutputBindingKind {
    fn default() -> Self {
        Self::Value(MidiValueOutput::default())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct MidiValueOutput {
    pub status: u8,
    pub data1: u8,
    pub min: u8,
    pub max: u8,
    pub active_value: u8,
    pub source: MidiValueSource,
}

impl Default for MidiValueOutput {
    fn default() -> Self {
        Self {
            status: 176,
            data1: 0,
            min: 0,
            max: 127,
            active_value: 127,
            source: MidiValueSource::SelectedSceneOpacity,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default)]
pub enum MidiValueSource {
    #[default]
    SelectedSceneOpacity,
    SelectedSceneInputDimmer,
    SelectedSceneBeatOffset,
    SelectedSceneIgnoreMainDimmer,
    SelectedSceneSetOffsetOnFlash,
    MainDimmer,
    BeatFlankPulse {
        start_beat: f32,
        end_beat: f32,
    },
    Blackout {
        inverted: bool,
        blink: bool,
    },
    SceneOpacity {
        target: MidiSceneTarget,
    },
    SceneInputDimmer {
        target: MidiSceneTarget,
    },
    SceneBeatOffset {
        target: MidiSceneTarget,
    },
    SceneIgnoreMainDimmer {
        target: MidiSceneTarget,
    },
    SceneSetOffsetOnFlash {
        target: MidiSceneTarget,
    },
    SceneActive {
        target: MidiSceneTarget,
    },
    SceneFlashed {
        target: MidiSceneTarget,
    },
    SceneEffectSettingF32 {
        target: MidiSceneTarget,
        effect_index: u8,
        setting_index: u8,
    },
}

impl Eq for MidiValueSource {}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum MidiColorSource {
    SceneColor {
        target: MidiSceneTarget,
    },
}

impl Default for MidiColorSource {
    fn default() -> Self {
        Self::SceneColor {
            target: MidiSceneTarget::Selected,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct MidiSceneColorValueOutput {
    pub source: MidiColorSource,
    pub mapping_name: String,
    pub status: u8,
    pub data1: u8,
}

impl Default for MidiSceneColorValueOutput {
    fn default() -> Self {
        Self {
            source: MidiColorSource::default(),
            mapping_name: String::new(),
            status: 144,
            data1: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct MidiNamedSceneColorMapping {
    pub name: String,
    pub active: MidiSceneColorMessageMap,
    pub inactive: MidiSceneColorMessageMap,
    pub flashed: MidiSceneColorMessageMap,
}

impl Default for MidiNamedSceneColorMapping {
    fn default() -> Self {
        let default_map = MidiSceneColorMessageMap::default();
        Self {
            name: "Color Mapping".to_owned(),
            active: default_map.clone(),
            inactive: default_map.clone(),
            flashed: default_map,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Default)]
pub struct MidiSceneColorMessage {
    pub value: u8,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct MidiSceneColorMessageMap {
    pub red: MidiSceneColorMessage,
    pub green: MidiSceneColorMessage,
    pub blue: MidiSceneColorMessage,
    pub white: MidiSceneColorMessage,
    pub orange: MidiSceneColorMessage,
    pub yellow: MidiSceneColorMessage,
    pub purple: MidiSceneColorMessage,
    pub pink: MidiSceneColorMessage,
    pub black: MidiSceneColorMessage,
}

impl Default for MidiSceneColorMessageMap {
    fn default() -> Self {
        Self {
            red: MidiSceneColorMessage { value: 5 },
            green: MidiSceneColorMessage { value: 21 },
            blue: MidiSceneColorMessage { value: 45 },
            white: MidiSceneColorMessage { value: 3 },
            orange: MidiSceneColorMessage { value: 9 },
            yellow: MidiSceneColorMessage { value: 13 },
            purple: MidiSceneColorMessage { value: 49 },
            pink: MidiSceneColorMessage { value: 53 },
            black: MidiSceneColorMessage { value: 0 },
        }
    }
}

impl MidiSceneColorMessageMap {
    pub fn message_for_color(&self, color: SceneInstanceColor) -> &MidiSceneColorMessage {
        match color {
            SceneInstanceColor::Red => &self.red,
            SceneInstanceColor::Green => &self.green,
            SceneInstanceColor::Blue => &self.blue,
            SceneInstanceColor::White => &self.white,
            SceneInstanceColor::Orange => &self.orange,
            SceneInstanceColor::Yellow => &self.yellow,
            SceneInstanceColor::Purple => &self.purple,
            SceneInstanceColor::Pink => &self.pink,
            SceneInstanceColor::Black => &self.black,
        }
    }
}

