use crate::storage::asset::midi_controller::{MidiController, MidiValueOutput, MidiValueSource};
use egui::{ComboBox, DragValue, Ui};

use super::{
    action_converters::scene_target_editor,
    value_source_converters::{
        MidiValueSourceKind, value_source_from_kind, value_source_kind, value_source_label,
        value_source_uses_active_value,
    },
};

pub(super) fn value_output_editor(
    ui: &mut Ui,
    value: &mut MidiValueOutput,
    dirty: &mut bool,
    controller_id: crate::storage::asset_id::AssetId<MidiController>,
    index: usize,
) -> (bool, bool) {
    let mut preview_active = false;
    let mut blackout_blink_preview_active = false;

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
                        MidiValueSourceKind::SelectedSceneDistributed,
                        "Selected Scene (Distributed 0-127)",
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
        let mut r = None;
        ui.horizontal(|ui| {
            ui.label("Value");
            let drag = ui.add(DragValue::new(&mut value.active_value).range(0..=127));
            if drag.changed() {
                *dirty = true;
            }
            r = Some(drag);
        });
        if let Some(r) = r {
            preview_active = preview_active || r.hovered() || r.dragged() || r.changed();
        }
    } else {
        let mut any_hovered = false;
        let mut any_dragged = false;
        ui.horizontal(|ui| {
            ui.label("Min");
            let r = ui.add(DragValue::new(&mut value.min).range(0..=127));
            if r.changed() {
                *dirty = true;
            }
            any_hovered |= r.hovered();
            any_dragged |= r.dragged();
            ui.label("Max");
            let r = ui.add(DragValue::new(&mut value.max).range(0..=127));
            if r.changed() {
                *dirty = true;
            }
            any_hovered |= r.hovered();
            any_dragged |= r.dragged();
        });
        preview_active = preview_active || any_hovered || any_dragged;
    }

    match &mut value.source {
        MidiValueSource::BeatFlankPulse {
            start_beat,
            end_beat,
            blackout_blink_value,
        } => {
            let mut any_hovered = false;
            let mut any_dragged = false;
            ui.horizontal(|ui| {
                ui.label("Start Beat");
                let r = ui.add(DragValue::new(start_beat).speed(0.01).range(0.0..=4.0));
                if r.changed() {
                    *dirty = true;
                }
                any_hovered |= r.hovered();
                any_dragged |= r.dragged();
                ui.label("End Beat");
                let r = ui.add(DragValue::new(end_beat).speed(0.01).range(0.0..=4.0));
                if r.changed() {
                    *dirty = true;
                }
                any_hovered |= r.hovered();
                any_dragged |= r.dragged();
            });
            preview_active = preview_active || any_hovered || any_dragged;
            ui.small("Inclusive beat range on 0.0..4.0, wrap supported");

            let mut blink_enabled = blackout_blink_value.is_some();
            if ui
                .checkbox(&mut blink_enabled, "Blink at 4 Hz on blackout")
                .changed()
            {
                *blackout_blink_value = if blink_enabled { Some(127) } else { None };
                *dirty = true;
            }
            if let Some(v) = blackout_blink_value {
                let mut blink_hovered = false;
                let mut blink_dragged = false;
                ui.horizontal(|ui| {
                    ui.label("Blackout blink value");
                    let r = ui.add(DragValue::new(v).range(1..=127));
                    if r.changed() {
                        *dirty = true;
                    }
                    blink_hovered |= r.hovered();
                    blink_dragged |= r.dragged();
                });
                blackout_blink_preview_active = blink_hovered || blink_dragged;
            }
        }
        MidiValueSource::Blackout { inverted, blink } => {
            let invert_response = ui.checkbox(inverted, "Invert (On when blackout is off)");
            if invert_response.changed() {
                *dirty = true;
            }
            let blink_response = ui.checkbox(blink, "Blink at 2 Hz when active");
            if blink_response.changed() {
                *dirty = true;
            }
            preview_active = preview_active
                || invert_response.hovered()
                || invert_response.changed()
                || blink_response.hovered()
                || blink_response.changed();
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
            let mut any_hovered = false;
            let mut any_dragged = false;
            ui.horizontal(|ui| {
                ui.label("Effect Index");
                let r = ui.add(DragValue::new(effect_index).range(0..=255));
                if r.changed() {
                    *dirty = true;
                }
                any_hovered |= r.hovered();
                any_dragged |= r.dragged();
                ui.label("Setting Index");
                let r = ui.add(DragValue::new(setting_index).range(0..=255));
                if r.changed() {
                    *dirty = true;
                }
                any_hovered |= r.hovered();
                any_dragged |= r.dragged();
            });
            preview_active = preview_active || any_hovered || any_dragged;
        }
        _ => {}
    }

    (preview_active, blackout_blink_preview_active)
}
