use crate::storage::asset::{
    midi_controller::{MidiController, MidiValueOutput, MidiValueSource},
};
use egui::{ComboBox, DragValue, Slider, Ui};
use egui_phosphor_icons::icons;

use super::{
    action_converters::scene_target_editor,
    iconized,
    value_source_converters::{
        value_source_from_kind, value_source_kind, value_source_label,
        value_source_uses_active_value, MidiValueSourceKind,
    },
    MidiControllerTestState,
};

pub(super) fn value_output_editor(
    ui: &mut Ui,
    value: &mut MidiValueOutput,
    test_state: &mut MidiControllerTestState,
    test_device_selected: bool,
    dirty: &mut bool,
    controller_id: crate::storage::asset_id::AssetId<MidiController>,
    index: usize,
) {
    ComboBox::new(format!("{}_value_source_{index}", controller_id), "")
        .selected_text(value_source_label(&value.source))
        .show_ui(ui, |ui| {
            let mut kind = value_source_kind(&value.source);
            if ui
                .selectable_value(
                    &mut kind,
                    MidiValueSourceKind::SelectedSceneOpacity,
                    "Selected Scene Opacity",
                )
                .changed()
                || ui
                    .selectable_value(
                        &mut kind,
                        MidiValueSourceKind::SelectedSceneInputDimmer,
                        "Selected Scene Input Dimmer",
                    )
                    .changed()
                || ui
                    .selectable_value(
                        &mut kind,
                        MidiValueSourceKind::SelectedSceneBeatOffset,
                        "Selected Scene Beat Offset",
                    )
                    .changed()
                || ui
                    .selectable_value(
                        &mut kind,
                        MidiValueSourceKind::SelectedSceneIgnoreMainDimmer,
                        "Selected Scene Ignore Main Dimmer",
                    )
                    .changed()
                || ui
                    .selectable_value(
                        &mut kind,
                        MidiValueSourceKind::SelectedSceneSetOffsetOnFlash,
                        "Selected Scene Set Offset On Flash",
                    )
                    .changed()
                || ui
                    .selectable_value(&mut kind, MidiValueSourceKind::MainDimmer, "Main Dimmer")
                    .changed()
                || ui
                    .selectable_value(&mut kind, MidiValueSourceKind::BeatFlank, "Beat Flank")
                    .changed()
                || ui
                    .selectable_value(&mut kind, MidiValueSourceKind::Blackout, "Blackout")
                    .changed()
                || ui
                    .selectable_value(
                        &mut kind,
                        MidiValueSourceKind::SceneOpacity,
                        "Scene Opacity",
                    )
                    .changed()
                || ui
                    .selectable_value(
                        &mut kind,
                        MidiValueSourceKind::SceneInputDimmer,
                        "Scene Input Dimmer",
                    )
                    .changed()
                || ui
                    .selectable_value(
                        &mut kind,
                        MidiValueSourceKind::SceneBeatOffset,
                        "Scene Beat Offset",
                    )
                    .changed()
                || ui
                    .selectable_value(
                        &mut kind,
                        MidiValueSourceKind::SceneIgnoreMainDimmer,
                        "Scene Ignore Main Dimmer",
                    )
                    .changed()
                || ui
                    .selectable_value(
                        &mut kind,
                        MidiValueSourceKind::SceneSetOffsetOnFlash,
                        "Scene Set Offset On Flash",
                    )
                    .changed()
                || ui
                    .selectable_value(&mut kind, MidiValueSourceKind::SceneActive, "Scene Active")
                    .changed()
                || ui
                    .selectable_value(
                        &mut kind,
                        MidiValueSourceKind::SceneFlashed,
                        "Scene Flashed",
                    )
                    .changed()
                || ui
                    .selectable_value(
                        &mut kind,
                        MidiValueSourceKind::SceneEffectSettingF32,
                        "Scene Effect Setting (n,n) f32",
                    )
                    .changed()
            {
                value.source = value_source_from_kind(kind, value.source.clone());
                *dirty = true;
            }
        });

    if value_source_uses_active_value(&value.source) {
        ui.horizontal(|ui| {
            ui.label("Value");
            if ui
                .add(DragValue::new(&mut value.active_value).range(0..=127))
                .changed()
            {
                *dirty = true;
            }
        });
    } else {
        ui.horizontal(|ui| {
            ui.label("Min");
            if ui.add(DragValue::new(&mut value.min).range(0..=127)).changed() {
                *dirty = true;
            }
            ui.label("Max");
            if ui.add(DragValue::new(&mut value.max).range(0..=127)).changed() {
                *dirty = true;
            }
        });
    }

    match &mut value.source {
        MidiValueSource::BeatFlankPulse {
            start_beat,
            end_beat,
        } => {
            ui.horizontal(|ui| {
                ui.label("Start Beat");
                if ui
                    .add(DragValue::new(start_beat).speed(0.01).range(0.0..=4.0))
                    .changed()
                {
                    *dirty = true;
                }
                ui.label("End Beat");
                if ui
                    .add(DragValue::new(end_beat).speed(0.01).range(0.0..=4.0))
                    .changed()
                {
                    *dirty = true;
                }
            });
            ui.small("Inclusive beat range on 0.0..4.0, wrap supported");
        }
        MidiValueSource::Blackout { inverted, blink } => {
            if ui.checkbox(inverted, "Invert (On when blackout is off)").changed() {
                *dirty = true;
            }
            if ui.checkbox(blink, "Blink at 2 Hz when active").changed() {
                *dirty = true;
            }
        }
        MidiValueSource::SceneOpacity { target }
        | MidiValueSource::SceneInputDimmer { target }
        | MidiValueSource::SceneBeatOffset { target }
        | MidiValueSource::SceneIgnoreMainDimmer { target }
        | MidiValueSource::SceneSetOffsetOnFlash { target }
        | MidiValueSource::SceneActive { target }
        | MidiValueSource::SceneFlashed { target } => {
            scene_target_editor(
                ui,
                target,
                dirty,
                format!("{}_value_target_{index}", controller_id),
            );
        }
        MidiValueSource::SceneEffectSettingF32 {
            target,
            effect_index,
            setting_index,
        } => {
            scene_target_editor(
                ui,
                target,
                dirty,
                format!("{}_value_target_{index}", controller_id),
            );
            ui.horizontal(|ui| {
                ui.label("Effect Index");
                if ui.add(DragValue::new(effect_index).range(0..=255)).changed() {
                    *dirty = true;
                }
                ui.label("Setting Index");
                if ui.add(DragValue::new(setting_index).range(0..=255)).changed() {
                    *dirty = true;
                }
            });
        }
        _ => {}
    }

    if test_device_selected && !value_source_uses_active_value(&value.source) {
        let test_value = test_state.value_output_overrides.entry(index).or_insert(value.max);
        ui.horizontal(|ui| {
            ui.label(iconized(ui, icons::FLASK, " Test Output"));
            ui.add(Slider::new(test_value, 0..=127).show_value(true));
        });
    } else {
        test_state.value_output_overrides.remove(&index);
    }
}
