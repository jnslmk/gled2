use crate::{
    midi::{
        learn::{LearnState, MidiLearnRequest, MidiLearnTarget},
        monitor,
    },
    storage::{
        asset::{
            Asset,
            midi_controller::{
                MidiColorChannelsOutput, MidiColorSource, MidiController, MidiInputAction,
                MidiControllerMapping, MidiInputBinding, MidiOutputBinding, MidiOutputBindingKind, MidiSceneTarget,
                MidiValueOutput, MidiValueSource,
            },
        },
        collections::Collections,
    },
    ui::{
        asset_tree::{AssetTree, TREE_WIDTH, TreeSelection},
        window_common::{default_viewport_builder, gled_window_frame},
    },
};
use chrono::{Local, TimeZone};
use egui::{Align, ComboBox, Context, DragValue, FontSelection, Id, Ui, Vec2, ViewportId};
use egui::epaint::text::{LayoutJob, TextFormat};
use egui_phosphor_icons::{Icon, icons};
use kanal::Receiver;
use std::collections::{HashMap, VecDeque};

const MAX_EVENTS: usize = 1000;

fn iconized(ui: &Ui, icon: Icon, text: &str) -> LayoutJob {
    let mut layout_job = LayoutJob::default();
    icon.regular().append_to(
        &mut layout_job,
        ui.style(),
        FontSelection::Default,
        Align::Center,
    );
    layout_job.append(text, 0.0, TextFormat::default());
    layout_job
}

#[derive(Default)]
pub struct MidiControllersWindow {
    open: bool,
    dirty: bool,
    tree: AssetTree<MidiController>,
    filter: String,
    events: VecDeque<monitor::MidiMonitorEvent>,
}

impl MidiControllersWindow {
    fn poll(&mut self, receiver: &Receiver<monitor::MidiMonitorEvent>) {
        while let Ok(Some(event)) = receiver.try_recv() {
            if self.events.len() >= MAX_EVENTS {
                self.events.pop_front();
            }
            self.events.push_back(event);
        }
    }

    pub fn diagnostics(&self) -> Vec<monitor::MidiPortDiagnostics> {
        let mut by_port: HashMap<String, monitor::MidiPortDiagnostics> = HashMap::new();
        for event in &self.events {
            let entry = by_port
                .entry(event.port_name.clone())
                .or_insert_with(|| monitor::MidiPortDiagnostics {
                    port_name: event.port_name.clone(),
                    ..Default::default()
                });
            match event.label.as_str() {
                "input connected" => entry.input_connected = true,
                "output connected" => entry.output_connected = true,
                "input connect failed" => entry.input_connect_failed += 1,
                "output connect failed" => entry.output_connect_failed += 1,
                "midi" => entry.message_count += 1,
                _ => {}
            }
        }
        let mut list: Vec<_> = by_port.into_values().collect();
        list.sort_by(|a, b| a.port_name.cmp(&b.port_name));
        list
    }
    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn update(
        &mut self,
        ctx: &Context,
        collections: &mut Collections,
        receiver: &Receiver<monitor::MidiMonitorEvent>,
        learn_state: &mut LearnState,
    ) {
        monitor::set_streaming_enabled(self.open);
        learn_state.set_streaming_enabled(self.open);
        self.poll(receiver);

        if !self.open {
            return;
        }

        let diagnostics = self.diagnostics();
        ctx.show_viewport_immediate(
            ViewportId(Id::new("midi controllers window")),
            default_viewport_builder()
                .with_inner_size(Vec2::new(1240.0, 740.0))
                .with_min_inner_size(Vec2::new(1240.0, 740.0)),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.open = false;
                        monitor::set_streaming_enabled(false);
                        learn_state.set_streaming_enabled(false);
                    }
                });

                gled_window_frame(ctx, "MIDI Controllers", |ui| {
                    egui::Panel::left("midi controllers tree")
                        .exact_size(TREE_WIDTH)
                        .resizable(false)
                        .show_inside(ui, |ui| {
                            if self.tree.show(
                                ui,
                                ui.make_persistent_id("midi_controllers_tree"),
                                collections,
                            ) {
                                self.dirty = false;
                            }
                        });

                    egui::Panel::right("midi_monitor_panel")
                        .exact_size(330.0)
                        .resizable(false)
                        .show_inside(ui, |ui| {
                            show_live_midi_monitor(ui, &mut self.events, &diagnostics);
                        });

                    egui::CentralPanel::default().show_inside(ui, |ui| {
                        self.tree.common_settings(ui, &mut self.dirty, collections);

                        if let TreeSelection::Asset(controller) = self.tree.selected() {
                            ui.separator();
                            midi_controller_editor(
                                ui,
                                controller,
                                &mut self.dirty,
                                collections,
                                &mut self.filter,
                                learn_state,
                            );
                        }
                    });
                });
            },
        );
    }

    pub fn open(&mut self) {
        self.open = true;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MidiInputActionKind {
    Tap,
    SetMainDimmer,
    SetBlackout,
    SetSpeedAdd,
    SetSpeedMultiply,
    SelectScene,
    ToggleSceneActive,
    SetSceneActive,
    SetSceneOpacity,
    SetSceneInputDimmer,
    SetSceneBeatOffset,
    SetSceneIgnoreMainDimmer,
    SetSceneSetOffsetOnFlash,
    SetSceneEffectSettingF32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MidiSceneTargetKind {
    Selected,
    Quick,
    Grid,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MidiValueSourceKind {
    SelectedSceneOpacity,
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
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MidiColorSourceKind {
    SelectedSceneColor,
    SceneColor,
}

fn midi_controller_editor(
    ui: &mut Ui,
    controller: &mut Asset<MidiController>,
    dirty: &mut bool,
    collections: &mut Collections,
    filter: &mut String,
    learn_state: &mut LearnState,
) {
    learn_state.flush_timeouts();
    apply_learn_captures(controller, dirty, collections, learn_state);

    ui.horizontal(|ui| {
        ui.label(icons::SLIDERS);
        ui.heading("Controller Type");
    });
    if ui
        .text_edit_singleline(&mut controller.data.controller_type)
        .changed()
    {
        *dirty = true;
    }

    ui.separator();
    ui.horizontal(|ui| {
        ui.label(iconized(ui, icons::FUNNEL, " Filter:"));
        ui.text_edit_singleline(filter);
        if !filter.is_empty() && ui.button(icons::X).clicked() {
            filter.clear();
        }
    });
    ui.add_space(4.0);

    let filter_str = filter.as_str();
    let controller_id = controller.id;
    let mapping = &mut controller.data.mapping;

    let mut remove_input = None;
    let mut pending_matching_output = Vec::new();
    let mut remove_output = None;

    ui.columns(2, |cols| {
        let (left, right) = cols.split_at_mut(1);
        let (ri, pmo) = render_input_column(
            &mut left[0],
            &mut mapping.input_bindings,
            filter_str,
            dirty,
            controller_id,
            learn_state,
        );
        remove_input = ri;
        pending_matching_output = pmo;
        remove_output = render_output_column(
            &mut right[0],
            &mut mapping.output_bindings,
            filter_str,
            dirty,
            controller_id,
            learn_state,
        );
    });

    for (name, status, data1, action) in pending_matching_output {
        if add_matching_output_binding(mapping, &name, status, data1, &action) {
            *dirty = true;
        }
    }

    if let Some(index) = remove_input {
        mapping.input_bindings.remove(index);
        *dirty = true;
    }

    if let Some(index) = remove_output {
        mapping.output_bindings.remove(index);
        *dirty = true;
    }
}

fn input_action_label(action: &MidiInputAction) -> &'static str {
    match action {
        MidiInputAction::Tap => "Tap",
        MidiInputAction::SetMainDimmer => "Set Main Dimmer",
        MidiInputAction::SetBlackout => "Set Blackout",
        MidiInputAction::SetSpeedAdd => "Set Speed Add",
        MidiInputAction::SetSpeedMultiply => "Set Speed Multiply",
        MidiInputAction::SelectScene { .. } => "Select Scene",
        MidiInputAction::ToggleSceneActive { .. } => "Toggle Scene Active",
        MidiInputAction::SetSceneActive { .. } => "Set Scene Active",
        MidiInputAction::SetSceneOpacity { .. } => "Set Scene Opacity",
        MidiInputAction::SetSceneInputDimmer { .. } => "Set Scene Input Dimmer",
        MidiInputAction::SetSceneBeatOffset { .. } => "Set Scene Beat Offset",
        MidiInputAction::SetSceneIgnoreMainDimmer { .. } => "Set Scene Ignore Main Dimmer",
        MidiInputAction::SetSceneSetOffsetOnFlash { .. } => "Set Scene Set Offset On Flash",
        MidiInputAction::SetSceneEffectSettingF32 { .. } => "Set Effect Setting (n,n) f32",
    }
}

fn input_binding_matches_filter(binding: &MidiInputBinding, filter: &str) -> bool {
    if filter.is_empty() {
        return true;
    }
    let f = filter.to_lowercase();
    binding.name.to_lowercase().contains(&f)
        || binding.trigger.status.to_string().contains(filter)
        || binding.trigger.data1.to_string().contains(filter)
        || input_action_label(&binding.action).to_lowercase().contains(&f)
}

fn output_binding_matches_filter(binding: &MidiOutputBinding, filter: &str) -> bool {
    if filter.is_empty() {
        return true;
    }
    let f = filter.to_lowercase();
    if binding.name.to_lowercase().contains(&f) {
        return true;
    }
    match &binding.kind {
        MidiOutputBindingKind::Value(v) => {
            v.status.to_string().contains(filter)
                || v.data1.to_string().contains(filter)
                || value_source_label(&v.source).to_lowercase().contains(&f)
        }
        MidiOutputBindingKind::ColorChannels(c) => {
            c.status.to_string().contains(filter)
                || c.data1.to_string().contains(filter)
                || color_source_label(&c.source).to_lowercase().contains(&f)
        }
    }
}

fn render_input_column(
    ui: &mut Ui,
    bindings: &mut Vec<MidiInputBinding>,
    filter: &str,
    dirty: &mut bool,
    controller_id: crate::storage::asset_id::AssetId<MidiController>,
    learn_state: &mut LearnState,
) -> (Option<usize>, Vec<(String, u8, u8, MidiInputAction)>) {
    let mut remove_input = None;
    let mut pending_matching_output = Vec::new();

    ui.horizontal(|ui| {
        ui.label(icons::ARROW_SQUARE_IN);
        ui.heading("Input Bindings");
    });
    ui.horizontal(|ui| {
        if ui
            .button(iconized(ui, icons::PLUS, " Add Input Binding"))
            .clicked()
        {
            bindings.push(MidiInputBinding::default());
            *dirty = true;
        }
    });

    egui::ScrollArea::vertical()
        .id_salt("input_bindings_scroll")
        .show(ui, |ui| {
            for (index, binding) in bindings.iter_mut().enumerate() {
                if !input_binding_matches_filter(binding, filter) {
                    continue;
                }
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("Input {}", index + 1));
                        let learning = learn_state.active_request().is_some_and(|request| {
                            request.controller_id == controller_id
                                && request.target == MidiLearnTarget::InputBinding(index)
                        });
                        if ui
                            .button(if learning {
                                iconized(ui, icons::MICROPHONE, " Waiting for MIDI...")
                            } else {
                                iconized(ui, icons::MICROPHONE, " Learn")
                            })
                            .clicked()
                        {
                            learn_state.arm(MidiLearnRequest {
                                controller_id,
                                target: MidiLearnTarget::InputBinding(index),
                            });
                        }
                        if ui.button(iconized(ui, icons::TRASH, " Remove")).clicked() {
                            remove_input = Some(index);
                        }
                    });

                    if ui.text_edit_singleline(&mut binding.name).changed() {
                        *dirty = true;
                    }

                    ui.horizontal(|ui| {
                        ui.label("Status");
                        if ui
                            .add(DragValue::new(&mut binding.trigger.status).range(0..=255))
                            .changed()
                        {
                            *dirty = true;
                        }
                        ui.label("Data1");
                        if ui
                            .add(DragValue::new(&mut binding.trigger.data1).range(0..=127))
                            .changed()
                        {
                            *dirty = true;
                        }
                    });

                    ui.label(format!(
                        "Detected Message: {}",
                        midi_status_label(binding.trigger.status)
                    ));

                    if ui
                        .checkbox(
                            &mut binding.trigger.match_data1,
                            "Match Data1 (disable for Pitch Bend style controls)",
                        )
                        .changed()
                    {
                        *dirty = true;
                    }

                    let mut action_kind = input_action_kind(&binding.action);
                    ComboBox::new(format!("{}_input_kind_{index}", controller_id), "Action")
                        .selected_text(format!("{action_kind:?}"))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut action_kind, MidiInputActionKind::Tap, "Tap");
                            ui.selectable_value(
                                &mut action_kind,
                                MidiInputActionKind::SetMainDimmer,
                                "Set Main Dimmer",
                            );
                            ui.selectable_value(
                                &mut action_kind,
                                MidiInputActionKind::SetBlackout,
                                "Set Blackout",
                            );
                            ui.selectable_value(
                                &mut action_kind,
                                MidiInputActionKind::SetSpeedAdd,
                                "Set Speed Add",
                            );
                            ui.selectable_value(
                                &mut action_kind,
                                MidiInputActionKind::SetSpeedMultiply,
                                "Set Speed Multiply",
                            );
                            ui.selectable_value(
                                &mut action_kind,
                                MidiInputActionKind::SelectScene,
                                "Select Scene",
                            );
                            ui.selectable_value(
                                &mut action_kind,
                                MidiInputActionKind::ToggleSceneActive,
                                "Toggle Scene Active",
                            );
                            ui.selectable_value(
                                &mut action_kind,
                                MidiInputActionKind::SetSceneActive,
                                "Set Scene Active",
                            );
                            ui.selectable_value(
                                &mut action_kind,
                                MidiInputActionKind::SetSceneOpacity,
                                "Set Scene Opacity",
                            );
                            ui.selectable_value(
                                &mut action_kind,
                                MidiInputActionKind::SetSceneInputDimmer,
                                "Set Scene Input Dimmer",
                            );
                            ui.selectable_value(
                                &mut action_kind,
                                MidiInputActionKind::SetSceneBeatOffset,
                                "Set Scene Beat Offset",
                            );
                            ui.selectable_value(
                                &mut action_kind,
                                MidiInputActionKind::SetSceneIgnoreMainDimmer,
                                "Set Scene Ignore Main Dimmer",
                            );
                            ui.selectable_value(
                                &mut action_kind,
                                MidiInputActionKind::SetSceneSetOffsetOnFlash,
                                "Set Scene Set Offset On Flash",
                            );
                            ui.selectable_value(
                                &mut action_kind,
                                MidiInputActionKind::SetSceneEffectSettingF32,
                                "Set Effect Setting (n,n) f32",
                            );
                        });

                    let before = binding.action.clone();
                    binding.action = input_action_from_kind(action_kind, binding.action.clone());
                    if binding.action != before {
                        *dirty = true;
                    }

                    match &mut binding.action {
                        MidiInputAction::SelectScene { target }
                        | MidiInputAction::ToggleSceneActive { target }
                        | MidiInputAction::SetSceneActive { target }
                        | MidiInputAction::SetSceneOpacity { target }
                        | MidiInputAction::SetSceneInputDimmer { target }
                        | MidiInputAction::SetSceneBeatOffset { target }
                        | MidiInputAction::SetSceneIgnoreMainDimmer { target }
                        | MidiInputAction::SetSceneSetOffsetOnFlash { target }
                        | MidiInputAction::SetSceneEffectSettingF32 { target, .. } => {
                            scene_target_editor(ui, target, dirty, format!("{}_input_target_{index}", controller_id));
                        }
                        _ => {}
                    }

                    if let MidiInputAction::SetSceneEffectSettingF32 {
                        effect_index,
                        setting_index,
                        ..
                    } = &mut binding.action
                    {
                        ui.horizontal(|ui| {
                            ui.label("Effect Index");
                            if ui
                                .add(DragValue::new(effect_index).range(0..=255))
                                .changed()
                            {
                                *dirty = true;
                            }
                            ui.label("Setting Index");
                            if ui
                                .add(DragValue::new(setting_index).range(0..=255))
                                .changed()
                            {
                                *dirty = true;
                            }
                        });
                    }

                    if ui
                        .button(iconized(ui, icons::PLUS, " Create Matching Output Binding"))
                        .clicked()
                    {
                        pending_matching_output.push((
                            binding.name.clone(),
                            binding.trigger.status,
                            binding.trigger.data1,
                            binding.action.clone(),
                        ));
                    }
                    if output_source_from_input_action(&binding.action).is_none() {
                        ui.small("This input action does not support automatic output binding creation.");
                    }
                });
            }
        });

    (remove_input, pending_matching_output)
}

fn render_output_column(
    ui: &mut Ui,
    bindings: &mut Vec<MidiOutputBinding>,
    filter: &str,
    dirty: &mut bool,
    controller_id: crate::storage::asset_id::AssetId<MidiController>,
    learn_state: &mut LearnState,
) -> Option<usize> {
    let mut remove_output = None;

    ui.horizontal(|ui| {
        ui.label(icons::ARROW_SQUARE_OUT);
        ui.heading("Output Bindings");
    });
    if ui
        .button(iconized(ui, icons::PLUS, " Add Output Binding"))
        .clicked()
    {
        bindings.push(MidiOutputBinding::default());
        *dirty = true;
    }

    egui::ScrollArea::vertical()
        .id_salt("output_bindings_scroll")
        .show(ui, |ui| {
            for (index, binding) in bindings.iter_mut().enumerate() {
                if !output_binding_matches_filter(binding, filter) {
                    continue;
                }
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("Output {}", index + 1));
                        let learning = learn_state.active_request().is_some_and(|request| {
                            request.controller_id == controller_id
                                && request.target == MidiLearnTarget::OutputBinding(index)
                        });
                        if ui
                            .button(if learning {
                                iconized(ui, icons::MICROPHONE, " Waiting for MIDI...")
                            } else {
                                iconized(ui, icons::MICROPHONE, " Learn")
                            })
                            .clicked()
                        {
                            learn_state.arm(MidiLearnRequest {
                                controller_id,
                                target: MidiLearnTarget::OutputBinding(index),
                            });
                        }
                        if ui.button(iconized(ui, icons::TRASH, " Remove")).clicked() {
                            remove_output = Some(index);
                        }
                    });

                    if ui.text_edit_singleline(&mut binding.name).changed() {
                        *dirty = true;
                    }

                    let mut kind = match binding.kind {
                        MidiOutputBindingKind::Value(_) => 0,
                        MidiOutputBindingKind::ColorChannels(_) => 1,
                    };

                    ComboBox::new(format!("{}_output_kind_{index}", controller_id), "Type")
                        .selected_text(if kind == 0 {
                            "Single Value"
                        } else {
                            "Color Channels"
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut kind, 0, "Single Value");
                            ui.selectable_value(&mut kind, 1, "Color Channels");
                        });

                    match kind {
                        0 => {
                            if !matches!(binding.kind, MidiOutputBindingKind::Value(_)) {
                                binding.kind = MidiOutputBindingKind::Value(MidiValueOutput::default());
                                *dirty = true;
                            }
                            if let MidiOutputBindingKind::Value(value) = &mut binding.kind {
                                value_output_editor(ui, value, dirty, controller_id, index);
                            }
                        }
                        _ => {
                            if !matches!(binding.kind, MidiOutputBindingKind::ColorChannels(_)) {
                                binding.kind = MidiOutputBindingKind::ColorChannels(
                                    MidiColorChannelsOutput::default(),
                                );
                                *dirty = true;
                            }
                            if let MidiOutputBindingKind::ColorChannels(colors) = &mut binding.kind {
                                color_output_editor(ui, colors, dirty);
                            }
                        }
                    }
                });
            }
        });

    remove_output
}

fn apply_learn_captures(
    controller: &mut Asset<MidiController>,
    dirty: &mut bool,
    collections: &mut Collections,
    learn_state: &mut LearnState,
) {
    let mut changed = false;
    while let Some(capture) = learn_state.pop_capture() {
        if capture.controller_id != controller.id {
            continue;
        }

        match capture.target {
            MidiLearnTarget::InputBinding(binding_index) => {
                if let Some(binding) = controller
                    .data
                    .mapping
                    .input_bindings
                    .get_mut(binding_index)
                {
                    binding.trigger = capture.trigger;
                    changed = true;
                }
            }
            MidiLearnTarget::OutputBinding(binding_index) => {
                if let Some(binding) = controller
                    .data
                    .mapping
                    .output_bindings
                    .get_mut(binding_index)
                {
                    match &mut binding.kind {
                        MidiOutputBindingKind::Value(value) => {
                            value.status = capture.trigger.status;
                            value.data1 = capture.trigger.data1;
                        }
                        MidiOutputBindingKind::ColorChannels(colors) => {
                            colors.status = capture.trigger.status;
                            colors.data1 = capture.trigger.data1;
                        }
                    }
                    changed = true;
                }
            }
        }
    }

    if changed {
        *dirty = true;
        controller.clone().save(collections);
    }
}

fn value_output_editor(
    ui: &mut Ui,
    value: &mut MidiValueOutput,
    dirty: &mut bool,
    controller_id: crate::storage::asset_id::AssetId<MidiController>,
    index: usize,
) {
    ui.horizontal(|ui| {
        ui.label("Status");
        if ui
            .add(DragValue::new(&mut value.status).range(0..=255))
            .changed()
        {
            *dirty = true;
        }
        ui.label("Data1");
        if ui
            .add(DragValue::new(&mut value.data1).range(0..=127))
            .changed()
        {
            *dirty = true;
        }
    });

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

    ComboBox::new(format!("{}_value_source_{index}", controller_id), "Source")
        .selected_text(value_source_label(&value.source))
        .show_ui(ui, |ui| {
            let mut kind = value_source_kind(&value.source);
            if ui.selectable_value(&mut kind, MidiValueSourceKind::SelectedSceneOpacity, "Selected Scene Opacity").changed()
                || ui.selectable_value(&mut kind, MidiValueSourceKind::SelectedSceneInputDimmer, "Selected Scene Input Dimmer").changed()
                || ui.selectable_value(&mut kind, MidiValueSourceKind::SelectedSceneBeatOffset, "Selected Scene Beat Offset").changed()
                || ui.selectable_value(&mut kind, MidiValueSourceKind::SelectedSceneIgnoreMainDimmer, "Selected Scene Ignore Main Dimmer").changed()
                || ui.selectable_value(&mut kind, MidiValueSourceKind::SelectedSceneSetOffsetOnFlash, "Selected Scene Set Offset On Flash").changed()
                || ui.selectable_value(&mut kind, MidiValueSourceKind::MainDimmer, "Main Dimmer").changed()
                || ui.selectable_value(&mut kind, MidiValueSourceKind::BeatFlank, "Beat Flank").changed()
                || ui.selectable_value(&mut kind, MidiValueSourceKind::Blackout, "Blackout").changed()
                || ui.selectable_value(&mut kind, MidiValueSourceKind::SceneOpacity, "Scene Opacity").changed()
                || ui.selectable_value(&mut kind, MidiValueSourceKind::SceneInputDimmer, "Scene Input Dimmer").changed()
                || ui.selectable_value(&mut kind, MidiValueSourceKind::SceneBeatOffset, "Scene Beat Offset").changed()
                || ui.selectable_value(&mut kind, MidiValueSourceKind::SceneIgnoreMainDimmer, "Scene Ignore Main Dimmer").changed()
                || ui.selectable_value(&mut kind, MidiValueSourceKind::SceneSetOffsetOnFlash, "Scene Set Offset On Flash").changed()
                || ui.selectable_value(&mut kind, MidiValueSourceKind::SceneActive, "Scene Active").changed()
                || ui.selectable_value(&mut kind, MidiValueSourceKind::SceneFlashed, "Scene Flashed").changed()
                || ui.selectable_value(&mut kind, MidiValueSourceKind::SceneEffectSettingF32, "Scene Effect Setting (n,n) f32").changed()
            {
                value.source = value_source_from_kind(kind, value.source.clone());
                *dirty = true;
            }
        });

    match &mut value.source {
        MidiValueSource::SceneOpacity { target }
        | MidiValueSource::SceneInputDimmer { target }
        | MidiValueSource::SceneBeatOffset { target }
        | MidiValueSource::SceneIgnoreMainDimmer { target }
        | MidiValueSource::SceneSetOffsetOnFlash { target }
        | MidiValueSource::SceneActive { target }
        | MidiValueSource::SceneFlashed { target } => {
            scene_target_editor(ui, target, dirty, format!("{}_value_target_{index}", controller_id));
        }
        MidiValueSource::SceneEffectSettingF32 {
            target,
            effect_index,
            setting_index,
        } => {
            scene_target_editor(ui, target, dirty, format!("{}_value_target_{index}", controller_id));
            ui.horizontal(|ui| {
                ui.label("Effect Index");
                if ui.add(DragValue::new(effect_index).range(0..=255)).changed() {
                    *dirty = true;
                }
                ui.label("Setting Index");
                if ui
                    .add(DragValue::new(setting_index).range(0..=255))
                    .changed()
                {
                    *dirty = true;
                }
            });
        }
        _ => {}
    }
}

fn color_output_editor(ui: &mut Ui, colors: &mut MidiColorChannelsOutput, dirty: &mut bool) {
    ui.horizontal(|ui| {
        ui.label("Status");
        if ui
            .add(DragValue::new(&mut colors.status).range(0..=255))
            .changed()
        {
            *dirty = true;
        }
        ui.label("Data1");
        if ui
            .add(DragValue::new(&mut colors.data1).range(0..=127))
            .changed()
        {
            *dirty = true;
        }
    });

    ComboBox::new(format!("color_source_{}_{}", colors.status, colors.data1), "Source")
        .selected_text(color_source_label(&colors.source))
        .show_ui(ui, |ui| {
            let mut kind = color_source_kind(&colors.source);
            if ui.selectable_value(&mut kind, MidiColorSourceKind::SelectedSceneColor, "Selected Scene Color").changed()
                || ui.selectable_value(&mut kind, MidiColorSourceKind::SceneColor, "Scene Color").changed()
            {
                colors.source = color_source_from_kind(kind, colors.source.clone());
                *dirty = true;
            }
        });

    if let MidiColorSource::SceneColor { target } = &mut colors.source {
        scene_target_editor(ui, target, dirty, format!("{}_{}_color_target", colors.status, colors.data1));
    }
}

fn input_action_kind(action: &MidiInputAction) -> MidiInputActionKind {
    match action {
        MidiInputAction::Tap => MidiInputActionKind::Tap,
        MidiInputAction::SetMainDimmer => MidiInputActionKind::SetMainDimmer,
        MidiInputAction::SetBlackout => MidiInputActionKind::SetBlackout,
        MidiInputAction::SetSpeedAdd => MidiInputActionKind::SetSpeedAdd,
        MidiInputAction::SetSpeedMultiply => MidiInputActionKind::SetSpeedMultiply,
        MidiInputAction::SelectScene { .. } => MidiInputActionKind::SelectScene,
        MidiInputAction::ToggleSceneActive { .. } => MidiInputActionKind::ToggleSceneActive,
        MidiInputAction::SetSceneActive { .. } => MidiInputActionKind::SetSceneActive,
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
    }
}

fn input_action_from_kind(kind: MidiInputActionKind, current: MidiInputAction) -> MidiInputAction {
    match kind {
        MidiInputActionKind::Tap => MidiInputAction::Tap,
        MidiInputActionKind::SetMainDimmer => MidiInputAction::SetMainDimmer,
        MidiInputActionKind::SetBlackout => MidiInputAction::SetBlackout,
        MidiInputActionKind::SetSpeedAdd => MidiInputAction::SetSpeedAdd,
        MidiInputActionKind::SetSpeedMultiply => MidiInputAction::SetSpeedMultiply,
        MidiInputActionKind::SelectScene => MidiInputAction::SelectScene {
            target: current_target_from_action(&current),
        },
        MidiInputActionKind::ToggleSceneActive => MidiInputAction::ToggleSceneActive {
            target: current_target_from_action(&current),
        },
        MidiInputActionKind::SetSceneActive => MidiInputAction::SetSceneActive {
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
    }
}

fn current_target_from_action(action: &MidiInputAction) -> MidiSceneTarget {
    match action {
        MidiInputAction::SelectScene { target }
        | MidiInputAction::ToggleSceneActive { target }
        | MidiInputAction::SetSceneActive { target }
        | MidiInputAction::SetSceneOpacity { target }
        | MidiInputAction::SetSceneInputDimmer { target }
        | MidiInputAction::SetSceneBeatOffset { target }
        | MidiInputAction::SetSceneIgnoreMainDimmer { target }
        | MidiInputAction::SetSceneSetOffsetOnFlash { target }
        | MidiInputAction::SetSceneEffectSettingF32 { target, .. } => target.clone(),
        _ => MidiSceneTarget::Selected,
    }
}

fn output_source_from_input_action(action: &MidiInputAction) -> Option<MidiValueSource> {
    match action {
        MidiInputAction::SetMainDimmer => Some(MidiValueSource::MainDimmer),
        MidiInputAction::SetBlackout => Some(MidiValueSource::Blackout),
        MidiInputAction::SetSceneActive { target } => Some(MidiValueSource::SceneActive {
            target: target.clone(),
        }),
        MidiInputAction::SetSceneOpacity { target } => Some(match target {
            MidiSceneTarget::Selected => MidiValueSource::SelectedSceneOpacity,
            _ => MidiValueSource::SceneOpacity {
                target: target.clone(),
            },
        }),
        MidiInputAction::SetSceneInputDimmer { target } => Some(match target {
            MidiSceneTarget::Selected => MidiValueSource::SelectedSceneInputDimmer,
            _ => MidiValueSource::SceneInputDimmer {
                target: target.clone(),
            },
        }),
        MidiInputAction::SetSceneBeatOffset { target } => Some(match target {
            MidiSceneTarget::Selected => MidiValueSource::SelectedSceneBeatOffset,
            _ => MidiValueSource::SceneBeatOffset {
                target: target.clone(),
            },
        }),
        MidiInputAction::SetSceneIgnoreMainDimmer { target } => Some(match target {
            MidiSceneTarget::Selected => MidiValueSource::SelectedSceneIgnoreMainDimmer,
            _ => MidiValueSource::SceneIgnoreMainDimmer {
                target: target.clone(),
            },
        }),
        MidiInputAction::SetSceneSetOffsetOnFlash { target } => Some(match target {
            MidiSceneTarget::Selected => MidiValueSource::SelectedSceneSetOffsetOnFlash,
            _ => MidiValueSource::SceneSetOffsetOnFlash {
                target: target.clone(),
            },
        }),
        MidiInputAction::SetSceneEffectSettingF32 {
            target,
            effect_index,
            setting_index,
        } => Some(MidiValueSource::SceneEffectSettingF32 {
            target: target.clone(),
            effect_index: *effect_index,
            setting_index: *setting_index,
        }),
        MidiInputAction::Tap
        | MidiInputAction::SetSpeedAdd
        | MidiInputAction::SetSpeedMultiply
        | MidiInputAction::SelectScene { .. }
        | MidiInputAction::ToggleSceneActive { .. } => None,
    }
}

fn add_matching_output_binding(
    mapping: &mut MidiControllerMapping,
    input_name: &str,
    input_status: u8,
    input_data1: u8,
    action: &MidiInputAction,
) -> bool {
    let Some(source) = output_source_from_input_action(action) else {
        return false;
    };

    let already_exists = mapping.output_bindings.iter().any(|binding| {
        matches!(
            &binding.kind,
            MidiOutputBindingKind::Value(value)
                if value.status == input_status
                    && value.data1 == input_data1
                    && value.source == source
        )
    });

    if already_exists {
        return false;
    }

    mapping.output_bindings.push(MidiOutputBinding {
        name: if input_name.is_empty() {
            "Output".to_owned()
        } else {
            format!("{} Output", input_name)
        },
        kind: MidiOutputBindingKind::Value(MidiValueOutput {
            status: input_status,
            data1: input_data1,
            min: 0,
            max: 127,
            source,
        }),
    });

    true
}

fn scene_target_editor(ui: &mut Ui, target: &mut MidiSceneTarget, dirty: &mut bool, id: String) {
    let mut kind = scene_target_kind(target);
    ComboBox::new(id, "Target")
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

fn value_source_kind(source: &MidiValueSource) -> MidiValueSourceKind {
    match source {
        MidiValueSource::SelectedSceneOpacity => MidiValueSourceKind::SelectedSceneOpacity,
        MidiValueSource::SelectedSceneInputDimmer => MidiValueSourceKind::SelectedSceneInputDimmer,
        MidiValueSource::SelectedSceneBeatOffset => MidiValueSourceKind::SelectedSceneBeatOffset,
        MidiValueSource::SelectedSceneIgnoreMainDimmer => {
            MidiValueSourceKind::SelectedSceneIgnoreMainDimmer
        }
        MidiValueSource::SelectedSceneSetOffsetOnFlash => {
            MidiValueSourceKind::SelectedSceneSetOffsetOnFlash
        }
        MidiValueSource::MainDimmer => MidiValueSourceKind::MainDimmer,
        MidiValueSource::BeatFlank => MidiValueSourceKind::BeatFlank,
        MidiValueSource::Blackout => MidiValueSourceKind::Blackout,
        MidiValueSource::SceneOpacity { .. } => MidiValueSourceKind::SceneOpacity,
        MidiValueSource::SceneInputDimmer { .. } => MidiValueSourceKind::SceneInputDimmer,
        MidiValueSource::SceneBeatOffset { .. } => MidiValueSourceKind::SceneBeatOffset,
        MidiValueSource::SceneIgnoreMainDimmer { .. } => {
            MidiValueSourceKind::SceneIgnoreMainDimmer
        }
        MidiValueSource::SceneSetOffsetOnFlash { .. } => {
            MidiValueSourceKind::SceneSetOffsetOnFlash
        }
        MidiValueSource::SceneActive { .. } => MidiValueSourceKind::SceneActive,
        MidiValueSource::SceneFlashed { .. } => MidiValueSourceKind::SceneFlashed,
        MidiValueSource::SceneEffectSettingF32 { .. } => MidiValueSourceKind::SceneEffectSettingF32,
    }
}

fn value_source_from_kind(kind: MidiValueSourceKind, current: MidiValueSource) -> MidiValueSource {
    match kind {
        MidiValueSourceKind::SelectedSceneOpacity => MidiValueSource::SelectedSceneOpacity,
        MidiValueSourceKind::SelectedSceneInputDimmer => MidiValueSource::SelectedSceneInputDimmer,
        MidiValueSourceKind::SelectedSceneBeatOffset => MidiValueSource::SelectedSceneBeatOffset,
        MidiValueSourceKind::SelectedSceneIgnoreMainDimmer => {
            MidiValueSource::SelectedSceneIgnoreMainDimmer
        }
        MidiValueSourceKind::SelectedSceneSetOffsetOnFlash => {
            MidiValueSource::SelectedSceneSetOffsetOnFlash
        }
        MidiValueSourceKind::MainDimmer => MidiValueSource::MainDimmer,
        MidiValueSourceKind::BeatFlank => MidiValueSource::BeatFlank,
        MidiValueSourceKind::Blackout => MidiValueSource::Blackout,
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
        | MidiValueSource::SceneEffectSettingF32 { target, .. } => target.clone(),
        _ => MidiSceneTarget::Selected,
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

fn value_source_label(source: &MidiValueSource) -> &'static str {
    match source {
        MidiValueSource::SelectedSceneOpacity => "Selected Scene Opacity",
        MidiValueSource::SelectedSceneInputDimmer => "Selected Scene Input Dimmer",
        MidiValueSource::SelectedSceneBeatOffset => "Selected Scene Beat Offset",
        MidiValueSource::SelectedSceneIgnoreMainDimmer => "Selected Scene Ignore Main Dimmer",
        MidiValueSource::SelectedSceneSetOffsetOnFlash => "Selected Scene Set Offset On Flash",
        MidiValueSource::MainDimmer => "Main Dimmer",
        MidiValueSource::BeatFlank => "Beat Flank",
        MidiValueSource::Blackout => "Blackout",
        MidiValueSource::SceneOpacity { .. } => "Scene Opacity",
        MidiValueSource::SceneInputDimmer { .. } => "Scene Input Dimmer",
        MidiValueSource::SceneBeatOffset { .. } => "Scene Beat Offset",
        MidiValueSource::SceneIgnoreMainDimmer { .. } => "Scene Ignore Main Dimmer",
        MidiValueSource::SceneSetOffsetOnFlash { .. } => "Scene Set Offset On Flash",
        MidiValueSource::SceneActive { .. } => "Scene Active",
        MidiValueSource::SceneFlashed { .. } => "Scene Flashed",
        MidiValueSource::SceneEffectSettingF32 { .. } => "Scene Effect Setting (n,n) f32",
    }
}

fn color_source_kind(source: &MidiColorSource) -> MidiColorSourceKind {
    match source {
        MidiColorSource::SelectedSceneColor => MidiColorSourceKind::SelectedSceneColor,
        MidiColorSource::SceneColor { .. } => MidiColorSourceKind::SceneColor,
    }
}

fn color_source_from_kind(kind: MidiColorSourceKind, current: MidiColorSource) -> MidiColorSource {
    match kind {
        MidiColorSourceKind::SelectedSceneColor => MidiColorSource::SelectedSceneColor,
        MidiColorSourceKind::SceneColor => MidiColorSource::SceneColor {
            target: current_target_from_color_source(&current),
        },
    }
}

fn current_target_from_color_source(source: &MidiColorSource) -> MidiSceneTarget {
    match source {
        MidiColorSource::SceneColor { target } => target.clone(),
        _ => MidiSceneTarget::Selected,
    }
}

fn color_source_label(source: &MidiColorSource) -> &'static str {
    match source {
        MidiColorSource::SelectedSceneColor => "Selected Scene Color",
        MidiColorSource::SceneColor { .. } => "Scene Color",
    }
}

fn show_live_midi_monitor(ui: &mut Ui, events: &mut VecDeque<monitor::MidiMonitorEvent>, diagnostics: &[monitor::MidiPortDiagnostics]) {
    ui.horizontal(|ui| {
        ui.label(icons::GAUGE);
        ui.heading("Diagnostics");
    });

    let diagnostics_height = ui.text_style_height(&egui::TextStyle::Body) * 3.0 + 8.0;
    egui::ScrollArea::vertical()
        .max_height(diagnostics_height)
        .id_salt("midi_diagnostics_scroll")
        .show(ui, |ui| {
            if diagnostics.is_empty() {
                ui.label("No MIDI ports observed yet.");
                return;
            }

            for diag in diagnostics {
                ui.label(format!(
                    "{} | in:{} out:{} | msgs:{} | in_fail:{} out_fail:{}",
                    diag.port_name,
                    if diag.input_connected { "yes" } else { "no" },
                    if diag.output_connected { "yes" } else { "no" },
                    diag.message_count,
                    diag.input_connect_failed,
                    diag.output_connect_failed
                ));
            }
        });
    ui.separator();

    ui.horizontal(|ui| {
        ui.label(icons::WAVEFORM);
        ui.heading("Live MIDI Monitor");
        if ui.button(iconized(ui, icons::TRASH, " Clear")).clicked() {
            events.clear();
        }
    });

    let events_slice: &[_] = events.make_contiguous();
    if events_slice.is_empty() {
        ui.label("No MIDI messages yet. Move a control on your controller.");
        return;
    }

    egui::ScrollArea::vertical()
        .max_height(ui.available_height())
        .stick_to_bottom(true)
        .id_salt("live_midi_monitor_scroll")
        .show(ui, |ui| {
            let log_text = events_slice
                .iter()
                .map(|event| {
                    let bytes = if event.bytes.is_empty() {
                        "-".to_owned()
                    } else {
                        event
                            .bytes
                            .iter()
                            .map(|b| format!("{b:03}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    };
                    let time = Local
                        .timestamp_millis_opt(event.timestamp_ms as i64)
                        .single()
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S%.3f").to_string())
                        .unwrap_or_else(|| event.timestamp_ms.to_string());
                    format!("{} | {} | {} | [{}]", time, event.port_name, event.label, bytes)
                })
                .collect::<Vec<_>>()
                .join("\n");

            ui.add(egui::Label::new(log_text).selectable(true).wrap());
        });
}

fn midi_status_label(status: u8) -> String {
    if status >= 0xF0 {
        return format!("System (0x{status:02X})");
    }

    let channel = (status & 0x0F) + 1;
    let kind = match status & 0xF0 {
        0x80 => "Note Off",
        0x90 => "Note On",
        0xA0 => "Poly Aftertouch",
        0xB0 => "Control Change",
        0xC0 => "Program Change",
        0xD0 => "Channel Pressure",
        0xE0 => "Pitch Bend",
        _ => "Unknown",
    };

    format!("{kind} (ch {channel}, 0x{status:02X})")
}
