use crate::storage::asset::midi_controller::{MidiColorSource, MidiSceneTarget, MidiValueSource};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum MidiValueSourceKind {
    SelectedSceneOpacity,
    SelectedSceneDistributed,
    SelectedSceneInputDimmer,
    SelectedSceneBeatOffset,
    SelectedSceneIgnoreMainDimmer,
    SelectedSceneSetOffsetOnFlash,
    MainDimmer,
    BeatFlank,
    Blackout,
    SceneOpacity,
    SceneInputDimmer,
    SceneBeatOffset,
    SceneIgnoreMainDimmer,
    SceneSetOffsetOnFlash,
    SceneActive,
    SceneFlashed,
    SceneEffectSettingF32,
    SceneEffectOpacity,
    SceneEffectColorShift,
    SceneEffectBeatProgression,
    SceneEffectBeatOffset,
    SceneEffectSpeedExponent,
    SceneEffectGroupIndex,
}

pub(super) fn value_source_kind(source: &MidiValueSource) -> MidiValueSourceKind {
    match source {
        MidiValueSource::SelectedSceneOpacity => MidiValueSourceKind::SelectedSceneOpacity,
        MidiValueSource::SelectedSceneDistributed => MidiValueSourceKind::SelectedSceneDistributed,
        MidiValueSource::SelectedSceneInputDimmer => MidiValueSourceKind::SelectedSceneInputDimmer,
        MidiValueSource::SelectedSceneBeatOffset => MidiValueSourceKind::SelectedSceneBeatOffset,
        MidiValueSource::SelectedSceneIgnoreMainDimmer => {
            MidiValueSourceKind::SelectedSceneIgnoreMainDimmer
        }
        MidiValueSource::SelectedSceneSetOffsetOnFlash => {
            MidiValueSourceKind::SelectedSceneSetOffsetOnFlash
        }
        MidiValueSource::MainDimmer => MidiValueSourceKind::MainDimmer,
        MidiValueSource::BeatFlankPulse { .. } => MidiValueSourceKind::BeatFlank,
        MidiValueSource::Blackout { .. } => MidiValueSourceKind::Blackout,
        MidiValueSource::SceneOpacity { .. } => MidiValueSourceKind::SceneOpacity,
        MidiValueSource::SceneInputDimmer { .. } => MidiValueSourceKind::SceneInputDimmer,
        MidiValueSource::SceneBeatOffset { .. } => MidiValueSourceKind::SceneBeatOffset,
        MidiValueSource::SceneIgnoreMainDimmer { .. } => MidiValueSourceKind::SceneIgnoreMainDimmer,
        MidiValueSource::SceneSetOffsetOnFlash { .. } => MidiValueSourceKind::SceneSetOffsetOnFlash,
        MidiValueSource::SceneActive { .. } => MidiValueSourceKind::SceneActive,
        MidiValueSource::SceneFlashed { .. } => MidiValueSourceKind::SceneFlashed,
        MidiValueSource::SceneEffectSettingF32 { .. } => MidiValueSourceKind::SceneEffectSettingF32,
        MidiValueSource::SceneEffectOpacity { .. } => MidiValueSourceKind::SceneEffectOpacity,
        MidiValueSource::SceneEffectColorShift { .. } => MidiValueSourceKind::SceneEffectColorShift,
        MidiValueSource::SceneEffectBeatProgression { .. } => MidiValueSourceKind::SceneEffectBeatProgression,
        MidiValueSource::SceneEffectBeatOffset { .. } => MidiValueSourceKind::SceneEffectBeatOffset,
        MidiValueSource::SceneEffectSpeedExponent { .. } => MidiValueSourceKind::SceneEffectSpeedExponent,
        MidiValueSource::SceneEffectGroupIndex { .. } => MidiValueSourceKind::SceneEffectGroupIndex,
    }
}

pub(super) fn value_source_from_kind(kind: MidiValueSourceKind, current: MidiValueSource) -> MidiValueSource {
    match kind {
        MidiValueSourceKind::SelectedSceneOpacity => MidiValueSource::SelectedSceneOpacity,
        MidiValueSourceKind::SelectedSceneDistributed => MidiValueSource::SelectedSceneDistributed,
        MidiValueSourceKind::SelectedSceneInputDimmer => MidiValueSource::SelectedSceneInputDimmer,
        MidiValueSourceKind::SelectedSceneBeatOffset => MidiValueSource::SelectedSceneBeatOffset,
        MidiValueSourceKind::SelectedSceneIgnoreMainDimmer => MidiValueSource::SelectedSceneIgnoreMainDimmer,
        MidiValueSourceKind::SelectedSceneSetOffsetOnFlash => MidiValueSource::SelectedSceneSetOffsetOnFlash,
        MidiValueSourceKind::MainDimmer => MidiValueSource::MainDimmer,
        MidiValueSourceKind::BeatFlank => match current {
            MidiValueSource::BeatFlankPulse { start_beat, end_beat, blackout_blink_value } => {
                MidiValueSource::BeatFlankPulse { start_beat, end_beat, blackout_blink_value }
            }
            _ => MidiValueSource::BeatFlankPulse {
                start_beat: 0.0,
                end_beat: 0.0,
                blackout_blink_value: None,
            },
        },
        MidiValueSourceKind::Blackout => match current {
            MidiValueSource::Blackout { inverted, blink } => MidiValueSource::Blackout {
                inverted,
                blink,
            },
            _ => MidiValueSource::Blackout {
                inverted: false,
                blink: false,
            },
        },
        MidiValueSourceKind::SceneOpacity => MidiValueSource::SceneOpacity {
            target: current_target_from_value_source(&current),
        },
        MidiValueSourceKind::SceneInputDimmer => MidiValueSource::SceneInputDimmer {
            target: current_target_from_value_source(&current),
        },
        MidiValueSourceKind::SceneBeatOffset => MidiValueSource::SceneBeatOffset {
            target: current_target_from_value_source(&current),
        },
        MidiValueSourceKind::SceneIgnoreMainDimmer => MidiValueSource::SceneIgnoreMainDimmer {
            target: current_target_from_value_source(&current),
        },
        MidiValueSourceKind::SceneSetOffsetOnFlash => MidiValueSource::SceneSetOffsetOnFlash {
            target: current_target_from_value_source(&current),
        },
        MidiValueSourceKind::SceneActive => MidiValueSource::SceneActive {
            target: current_target_from_value_source(&current),
        },
        MidiValueSourceKind::SceneFlashed => MidiValueSource::SceneFlashed {
            target: current_target_from_value_source(&current),
        },
        MidiValueSourceKind::SceneEffectSettingF32 => {
            let (effect_index, setting_index) = current_effect_setting_from_value_source(&current);
            MidiValueSource::SceneEffectSettingF32 {
                target: current_target_from_value_source(&current),
                effect_index,
                setting_index,
            }
        }
        MidiValueSourceKind::SceneEffectOpacity => MidiValueSource::SceneEffectOpacity {
            target: current_target_from_value_source(&current),
            effect_index: current_effect_index_from_value_source(&current),
        },
        MidiValueSourceKind::SceneEffectColorShift => MidiValueSource::SceneEffectColorShift {
            target: current_target_from_value_source(&current),
            effect_index: current_effect_index_from_value_source(&current),
        },
        MidiValueSourceKind::SceneEffectBeatProgression => MidiValueSource::SceneEffectBeatProgression {
            target: current_target_from_value_source(&current),
            effect_index: current_effect_index_from_value_source(&current),
        },
        MidiValueSourceKind::SceneEffectBeatOffset => MidiValueSource::SceneEffectBeatOffset {
            target: current_target_from_value_source(&current),
            effect_index: current_effect_index_from_value_source(&current),
        },
        MidiValueSourceKind::SceneEffectSpeedExponent => MidiValueSource::SceneEffectSpeedExponent {
            target: current_target_from_value_source(&current),
            effect_index: current_effect_index_from_value_source(&current),
        },
        MidiValueSourceKind::SceneEffectGroupIndex => MidiValueSource::SceneEffectGroupIndex {
            target: current_target_from_value_source(&current),
            effect_index: current_effect_index_from_value_source(&current),
        },
    }
}

fn current_target_from_value_source(source: &MidiValueSource) -> MidiSceneTarget {
    match source {
        MidiValueSource::SceneOpacity { target }
        | MidiValueSource::SceneInputDimmer { target }
        | MidiValueSource::SceneBeatOffset { target }
        | MidiValueSource::SceneIgnoreMainDimmer { target }
        | MidiValueSource::SceneSetOffsetOnFlash { target }
        | MidiValueSource::SceneActive { target }
        | MidiValueSource::SceneFlashed { target }
        | MidiValueSource::SceneEffectSettingF32 { target, .. }
        | MidiValueSource::SceneEffectOpacity { target, .. }
        | MidiValueSource::SceneEffectColorShift { target, .. }
        | MidiValueSource::SceneEffectBeatProgression { target, .. }
        | MidiValueSource::SceneEffectBeatOffset { target, .. }
        | MidiValueSource::SceneEffectSpeedExponent { target, .. }
        | MidiValueSource::SceneEffectGroupIndex { target, .. } => target.clone(),
        _ => MidiSceneTarget::Selected,
    }
}

fn current_effect_index_from_value_source(source: &MidiValueSource) -> u8 {
    match source {
        MidiValueSource::SceneEffectSettingF32 { effect_index, .. }
        | MidiValueSource::SceneEffectOpacity { effect_index, .. }
        | MidiValueSource::SceneEffectColorShift { effect_index, .. }
        | MidiValueSource::SceneEffectBeatProgression { effect_index, .. }
        | MidiValueSource::SceneEffectBeatOffset { effect_index, .. }
        | MidiValueSource::SceneEffectSpeedExponent { effect_index, .. }
        | MidiValueSource::SceneEffectGroupIndex { effect_index, .. } => *effect_index,
        _ => 0,
    }
}

fn current_effect_setting_from_value_source(source: &MidiValueSource) -> (u8, u8) {
    match source {
        MidiValueSource::SceneEffectSettingF32 {
            effect_index,
            setting_index,
            ..
        } => (*effect_index, *setting_index),
        _ => (0, 0),
    }
}

pub(super) fn value_source_label(source: &MidiValueSource) -> &'static str {
    match source {
        MidiValueSource::SelectedSceneOpacity => "Selected Scene Opacity",
        MidiValueSource::SelectedSceneDistributed => "Selected Scene (Distributed 0-127)",
        MidiValueSource::SelectedSceneInputDimmer => "Selected Scene Input Dimmer",
        MidiValueSource::SelectedSceneBeatOffset => "Selected Scene Beat Offset",
        MidiValueSource::SelectedSceneIgnoreMainDimmer => "Selected Scene Ignore Main Dimmer",
        MidiValueSource::SelectedSceneSetOffsetOnFlash => "Selected Scene Set Offset On Flash",
        MidiValueSource::MainDimmer => "Main Dimmer",
        MidiValueSource::BeatFlankPulse { .. } => "Beat Flank",
        MidiValueSource::Blackout { .. } => "Blackout",
        MidiValueSource::SceneOpacity { .. } => "Scene Opacity",
        MidiValueSource::SceneInputDimmer { .. } => "Scene Input Dimmer",
        MidiValueSource::SceneBeatOffset { .. } => "Scene Beat Offset",
        MidiValueSource::SceneIgnoreMainDimmer { .. } => "Scene Ignore Main Dimmer",
        MidiValueSource::SceneSetOffsetOnFlash { .. } => "Scene Set Offset On Flash",
        MidiValueSource::SceneActive { .. } => "Scene Active",
        MidiValueSource::SceneFlashed { .. } => "Scene Flashed",
        MidiValueSource::SceneEffectSettingF32 { .. } => "Scene Effect Setting (n,n) f32",
        MidiValueSource::SceneEffectOpacity { .. } => "Effect Opacity",
        MidiValueSource::SceneEffectColorShift { .. } => "Effect Color Shift",
        MidiValueSource::SceneEffectBeatProgression { .. } => "Effect Beat Progression",
        MidiValueSource::SceneEffectBeatOffset { .. } => "Effect Beat Offset",
        MidiValueSource::SceneEffectSpeedExponent { .. } => "Effect Speed Exponent",
        MidiValueSource::SceneEffectGroupIndex { .. } => "Effect Group Index",
    }
}

pub(super) fn value_source_uses_active_value(source: &MidiValueSource) -> bool {
    match source {
        MidiValueSource::SelectedSceneIgnoreMainDimmer
        | MidiValueSource::SelectedSceneSetOffsetOnFlash
        | MidiValueSource::BeatFlankPulse { .. }
        | MidiValueSource::Blackout { .. }
        | MidiValueSource::SceneIgnoreMainDimmer { .. }
        | MidiValueSource::SceneSetOffsetOnFlash { .. }
        | MidiValueSource::SceneActive { .. }
        | MidiValueSource::SceneFlashed { .. } => true,
        MidiValueSource::SelectedSceneOpacity
        | MidiValueSource::SelectedSceneDistributed
        | MidiValueSource::SelectedSceneInputDimmer
        | MidiValueSource::SelectedSceneBeatOffset
        | MidiValueSource::MainDimmer
        | MidiValueSource::SceneOpacity { .. }
        | MidiValueSource::SceneInputDimmer { .. }
        | MidiValueSource::SceneBeatOffset { .. }
        | MidiValueSource::SceneEffectSettingF32 { .. }
        | MidiValueSource::SceneEffectOpacity { .. }
        | MidiValueSource::SceneEffectColorShift { .. }
        | MidiValueSource::SceneEffectBeatProgression { .. }
        | MidiValueSource::SceneEffectBeatOffset { .. }
        | MidiValueSource::SceneEffectSpeedExponent { .. }
        | MidiValueSource::SceneEffectGroupIndex { .. } => false,
    }
}

pub(super) fn color_source_label(source: &MidiColorSource) -> &'static str {
    match source {
        MidiColorSource::SceneColor { .. } => "Scene Color",
    }
}
