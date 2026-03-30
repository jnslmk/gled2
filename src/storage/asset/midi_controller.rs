use super::AssetTrait;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(default)]
pub struct MidiController {
    #[serde(default = "default_controller_type")]
    pub controller_type: String,
    #[serde(default)]
    pub mapping: MidiControllerMapping,
}

fn default_controller_type() -> String {
    MidiController::default().controller_type
}

impl Default for MidiController {
    fn default() -> Self {
        Self {
            controller_type: "Generic MIDI Controller".to_owned(),
            mapping: MidiControllerMapping::default(),
        }
    }
}

impl AssetTrait for MidiController {
    const DIR_NAME: &'static str = "midi_controllers";
    const NAME: &'static str = "MIDI Controller";
    const SHOW_NAME_IF_SELECTED: bool = true;
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Default)]
#[serde(default)]
pub struct MidiControllerMapping {
    pub input_bindings: Vec<MidiInputBinding>,
    pub output_bindings: Vec<MidiOutputBinding>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(default)]
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
#[serde(default)]
pub struct MidiTrigger {
    pub status: u8,
    pub data1: u8,
    #[serde(default = "default_true")]
    pub match_data1: bool,
}

const fn default_true() -> bool {
    true
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
    SelectScene { target: MidiSceneTarget },
    ToggleSceneActive { target: MidiSceneTarget },
    SetSceneActive { target: MidiSceneTarget },
    SetSceneOpacity { target: MidiSceneTarget },
    SetSceneInputDimmer { target: MidiSceneTarget },
    SetSceneBeatOffset { target: MidiSceneTarget },
    SetSceneIgnoreMainDimmer { target: MidiSceneTarget },
    SetSceneSetOffsetOnFlash { target: MidiSceneTarget },
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
    Quick { index: u8 },
    Grid { row: u8, col: u8 },
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(default)]
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
    ColorChannels(MidiColorChannelsOutput),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(default)]
pub struct MidiValueOutput {
    pub status: u8,
    pub data1: u8,
    pub min: u8,
    pub max: u8,
    pub source: MidiValueSource,
}

impl Default for MidiValueOutput {
    fn default() -> Self {
        Self {
            status: 176,
            data1: 0,
            min: 0,
            max: 127,
            source: MidiValueSource::SelectedSceneOpacity,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Default)]
pub enum MidiValueSource {
    #[default]
    SelectedSceneOpacity,
    SelectedSceneInputDimmer,
    SelectedSceneBeatOffset,
    SelectedSceneIgnoreMainDimmer,
    SelectedSceneSetOffsetOnFlash,
    MainDimmer,
    BeatFlank,
    Blackout,
    SceneOpacity { target: MidiSceneTarget },
    SceneInputDimmer { target: MidiSceneTarget },
    SceneBeatOffset { target: MidiSceneTarget },
    SceneIgnoreMainDimmer { target: MidiSceneTarget },
    SceneSetOffsetOnFlash { target: MidiSceneTarget },
    SceneActive { target: MidiSceneTarget },
    SceneFlashed { target: MidiSceneTarget },
    SceneEffectSettingF32 {
        target: MidiSceneTarget,
        effect_index: u8,
        setting_index: u8,
    },
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(default)]
pub struct MidiColorChannelsOutput {
    pub status: u8,
    pub data1: u8,
    pub source: MidiColorSource,
}

impl Default for MidiColorChannelsOutput {
    fn default() -> Self {
        Self {
            status: 144,
            data1: 0,
            source: MidiColorSource::SelectedSceneColor,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Default)]
pub enum MidiColorSource {
    #[default]
    SelectedSceneColor,
    SceneColor { target: MidiSceneTarget },
}
