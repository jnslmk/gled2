use crate::storage::asset::midi_controller::{MidiInputAction, MidiSceneTarget};
use egui::{ComboBox, DragValue, Ui};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MidiInputActionKind {
    Tap,
    SetMainDimmer,
    SetBlackout,
    SetSpeedAdd,
    SetSpeedMultiply,
    SelectSceneDistributed,
    SelectScene,
    ToggleSceneActive,
    SetSceneActive,
    FlashScene,
    SetSceneOpacity,
    SetSceneInputDimmer,
    SetSceneBeatOffset,
    SetSceneIgnoreMainDimmer,
    SetSceneSetOffsetOnFlash,
    SetSceneEffectSettingF32,
    SetSceneEffectOpacity,
    SetSceneEffectColorShift,
    SetSceneEffectBeatProgression,
    SetSceneEffectBeatOffset,
    SetSceneEffectSpeedExponent,
    SetSceneEffectGroupIndex,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MidiSceneTargetKind {
    Selected,
    Quick,
    Grid,
}

pub(super) fn input_action_kind(action: &MidiInputAction) -> MidiInputActionKind {
    match action {
        MidiInputAction::Tap => MidiInputActionKind::Tap,
        MidiInputAction::SetMainDimmer => MidiInputActionKind::SetMainDimmer,
        MidiInputAction::SetBlackout => MidiInputActionKind::SetBlackout,
        MidiInputAction::SetSpeedAdd => MidiInputActionKind::SetSpeedAdd,
        MidiInputAction::SetSpeedMultiply => MidiInputActionKind::SetSpeedMultiply,
        MidiInputAction::SelectSceneDistributed => MidiInputActionKind::SelectSceneDistributed,
        MidiInputAction::SelectScene { .. } => MidiInputActionKind::SelectScene,
        MidiInputAction::ToggleSceneActive { .. } => MidiInputActionKind::ToggleSceneActive,
        MidiInputAction::SetSceneActive { .. } => MidiInputActionKind::SetSceneActive,
        MidiInputAction::FlashScene { .. } => MidiInputActionKind::FlashScene,
        MidiInputAction::SetSceneOpacity { .. } => MidiInputActionKind::SetSceneOpacity,
        MidiInputAction::SetSceneInputDimmer { .. } => MidiInputActionKind::SetSceneInputDimmer,
        MidiInputAction::SetSceneBeatOffset { .. } => MidiInputActionKind::SetSceneBeatOffset,
        MidiInputAction::SetSceneIgnoreMainDimmer { .. } => {
            MidiInputActionKind::SetSceneIgnoreMainDimmer
        }
        MidiInputAction::SetSceneSetOffsetOnFlash { .. } => {
            MidiInputActionKind::SetSceneSetOffsetOnFlash
        }
        MidiInputAction::SetSceneEffectSettingF32 { .. } => {
            MidiInputActionKind::SetSceneEffectSettingF32
        }
        MidiInputAction::SetSceneEffectOpacity { .. } => MidiInputActionKind::SetSceneEffectOpacity,
        MidiInputAction::SetSceneEffectColorShift { .. } => {
            MidiInputActionKind::SetSceneEffectColorShift
        }
        MidiInputAction::SetSceneEffectBeatProgression { .. } => {
            MidiInputActionKind::SetSceneEffectBeatProgression
        }
        MidiInputAction::SetSceneEffectBeatOffset { .. } => {
            MidiInputActionKind::SetSceneEffectBeatOffset
        }
        MidiInputAction::SetSceneEffectSpeedExponent { .. } => {
            MidiInputActionKind::SetSceneEffectSpeedExponent
        }
        MidiInputAction::SetSceneEffectGroupIndex { .. } => {
            MidiInputActionKind::SetSceneEffectGroupIndex
        }
    }
}

pub(super) fn input_action_from_kind(
    kind: MidiInputActionKind,
    current: MidiInputAction,
) -> MidiInputAction {
    match kind {
        MidiInputActionKind::Tap => MidiInputAction::Tap,
        MidiInputActionKind::SetMainDimmer => MidiInputAction::SetMainDimmer,
        MidiInputActionKind::SetBlackout => MidiInputAction::SetBlackout,
        MidiInputActionKind::SetSpeedAdd => MidiInputAction::SetSpeedAdd,
        MidiInputActionKind::SetSpeedMultiply => MidiInputAction::SetSpeedMultiply,
        MidiInputActionKind::SelectSceneDistributed => MidiInputAction::SelectSceneDistributed,
        MidiInputActionKind::SelectScene => MidiInputAction::SelectScene {
            target: current_target_from_action(&current),
        },
        MidiInputActionKind::ToggleSceneActive => MidiInputAction::ToggleSceneActive {
            target: current_target_from_action(&current),
        },
        MidiInputActionKind::SetSceneActive => MidiInputAction::SetSceneActive {
            target: current_target_from_action(&current),
        },
        MidiInputActionKind::FlashScene => MidiInputAction::FlashScene {
            target: current_target_from_action(&current),
        },
        MidiInputActionKind::SetSceneOpacity => MidiInputAction::SetSceneOpacity {
            target: current_target_from_action(&current),
        },
        MidiInputActionKind::SetSceneInputDimmer => MidiInputAction::SetSceneInputDimmer {
            target: current_target_from_action(&current),
        },
        MidiInputActionKind::SetSceneBeatOffset => MidiInputAction::SetSceneBeatOffset {
            target: current_target_from_action(&current),
        },
        MidiInputActionKind::SetSceneIgnoreMainDimmer => {
            MidiInputAction::SetSceneIgnoreMainDimmer {
                target: current_target_from_action(&current),
            }
        }
        MidiInputActionKind::SetSceneSetOffsetOnFlash => {
            MidiInputAction::SetSceneSetOffsetOnFlash {
                target: current_target_from_action(&current),
            }
        }
        MidiInputActionKind::SetSceneEffectSettingF32 => {
            let (effect_index, setting_index) = match &current {
                MidiInputAction::SetSceneEffectSettingF32 {
                    effect_index,
                    setting_index,
                    ..
                } => (*effect_index, *setting_index),
                _ => (0, 0),
            };
            MidiInputAction::SetSceneEffectSettingF32 {
                target: current_target_from_action(&current),
                effect_index,
                setting_index,
            }
        }
        MidiInputActionKind::SetSceneEffectOpacity => {
            let effect_index = current_effect_index_from_action(&current);
            MidiInputAction::SetSceneEffectOpacity {
                target: current_target_from_action(&current),
                effect_index,
            }
        }
        MidiInputActionKind::SetSceneEffectColorShift => {
            let effect_index = current_effect_index_from_action(&current);
            MidiInputAction::SetSceneEffectColorShift {
                target: current_target_from_action(&current),
                effect_index,
            }
        }
        MidiInputActionKind::SetSceneEffectBeatProgression => {
            let effect_index = current_effect_index_from_action(&current);
            MidiInputAction::SetSceneEffectBeatProgression {
                target: current_target_from_action(&current),
                effect_index,
            }
        }
        MidiInputActionKind::SetSceneEffectBeatOffset => {
            let effect_index = current_effect_index_from_action(&current);
            MidiInputAction::SetSceneEffectBeatOffset {
                target: current_target_from_action(&current),
                effect_index,
            }
        }
        MidiInputActionKind::SetSceneEffectSpeedExponent => {
            let effect_index = current_effect_index_from_action(&current);
            MidiInputAction::SetSceneEffectSpeedExponent {
                target: current_target_from_action(&current),
                effect_index,
            }
        }
        MidiInputActionKind::SetSceneEffectGroupIndex => {
            let effect_index = current_effect_index_from_action(&current);
            MidiInputAction::SetSceneEffectGroupIndex {
                target: current_target_from_action(&current),
                effect_index,
            }
        }
    }
}

fn current_target_from_action(action: &MidiInputAction) -> MidiSceneTarget {
    match action {
        MidiInputAction::SelectScene { target }
        | MidiInputAction::ToggleSceneActive { target }
        | MidiInputAction::SetSceneActive { target }
        | MidiInputAction::FlashScene { target }
        | MidiInputAction::SetSceneOpacity { target }
        | MidiInputAction::SetSceneInputDimmer { target }
        | MidiInputAction::SetSceneBeatOffset { target }
        | MidiInputAction::SetSceneIgnoreMainDimmer { target }
        | MidiInputAction::SetSceneSetOffsetOnFlash { target }
        | MidiInputAction::SetSceneEffectSettingF32 { target, .. }
        | MidiInputAction::SetSceneEffectOpacity { target, .. }
        | MidiInputAction::SetSceneEffectColorShift { target, .. }
        | MidiInputAction::SetSceneEffectBeatProgression { target, .. }
        | MidiInputAction::SetSceneEffectBeatOffset { target, .. }
        | MidiInputAction::SetSceneEffectSpeedExponent { target, .. }
        | MidiInputAction::SetSceneEffectGroupIndex { target, .. } => target.clone(),
        _ => MidiSceneTarget::Selected,
    }
}

fn current_effect_index_from_action(action: &MidiInputAction) -> u8 {
    match action {
        MidiInputAction::SetSceneEffectSettingF32 { effect_index, .. }
        | MidiInputAction::SetSceneEffectOpacity { effect_index, .. }
        | MidiInputAction::SetSceneEffectColorShift { effect_index, .. }
        | MidiInputAction::SetSceneEffectBeatProgression { effect_index, .. }
        | MidiInputAction::SetSceneEffectBeatOffset { effect_index, .. }
        | MidiInputAction::SetSceneEffectSpeedExponent { effect_index, .. }
        | MidiInputAction::SetSceneEffectGroupIndex { effect_index, .. } => *effect_index,
        _ => 0,
    }
}

pub(super) fn scene_target_editor(
    ui: &mut Ui,
    target: &mut MidiSceneTarget,
    dirty: &mut bool,
    id: String,
) {
    let mut kind = scene_target_kind(target);
    ComboBox::new(id, "")
        .selected_text(scene_target_label(target))
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut kind, MidiSceneTargetKind::Selected, "Selected Scene");
            ui.selectable_value(&mut kind, MidiSceneTargetKind::Quick, "Quick Scene");
            ui.selectable_value(&mut kind, MidiSceneTargetKind::Grid, "Grid Scene");
        });

    let next = scene_target_from_kind(kind, target.clone());
    if *target != next {
        *target = next;
        *dirty = true;
    }

    match target {
        MidiSceneTarget::Quick { index } => {
            ui.horizontal(|ui| {
                ui.label("Quick Scene Index");
                if ui.add(DragValue::new(index).range(0..=7)).changed() {
                    *dirty = true;
                }
            });
        }
        MidiSceneTarget::Grid { row, col } => {
            ui.horizontal(|ui| {
                ui.label("Row");
                if ui.add(DragValue::new(row).range(0..=5)).changed() {
                    *dirty = true;
                }
                ui.label("Col");
                if ui.add(DragValue::new(col).range(0..=7)).changed() {
                    *dirty = true;
                }
            });
        }
        MidiSceneTarget::Selected => {}
    }
}

fn scene_target_kind(target: &MidiSceneTarget) -> MidiSceneTargetKind {
    match target {
        MidiSceneTarget::Selected => MidiSceneTargetKind::Selected,
        MidiSceneTarget::Quick { .. } => MidiSceneTargetKind::Quick,
        MidiSceneTarget::Grid { .. } => MidiSceneTargetKind::Grid,
    }
}

fn scene_target_from_kind(kind: MidiSceneTargetKind, current: MidiSceneTarget) -> MidiSceneTarget {
    match kind {
        MidiSceneTargetKind::Selected => MidiSceneTarget::Selected,
        MidiSceneTargetKind::Quick => match current {
            MidiSceneTarget::Quick { index } => MidiSceneTarget::Quick { index },
            _ => MidiSceneTarget::Quick { index: 0 },
        },
        MidiSceneTargetKind::Grid => match current {
            MidiSceneTarget::Grid { row, col } => MidiSceneTarget::Grid { row, col },
            _ => MidiSceneTarget::Grid { row: 0, col: 0 },
        },
    }
}

fn scene_target_label(target: &MidiSceneTarget) -> &'static str {
    match target {
        MidiSceneTarget::Selected => "Selected Scene",
        MidiSceneTarget::Quick { .. } => "Quick Scene",
        MidiSceneTarget::Grid { .. } => "Grid Scene",
    }
}
