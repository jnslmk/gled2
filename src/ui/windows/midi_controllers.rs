use crate::{
    midi::{
        learn::LearnState,
        monitor,
        runtime,
    },
    storage::{
        asset::{
            Asset,
            midi_controller::MidiController,
        },
        collections::Collections,
    },
    ui::{
        asset_tree::{AssetTree, TREE_WIDTH, TreeSelection},
        window_common::{default_viewport_builder, gled_window_frame},
    },
};
use egui::epaint::text::{LayoutJob, TextFormat};
use egui::{Align, Context, FontSelection, Id, Ui, Vec2, ViewportId};
use egui_phosphor_icons::{Icon, icons};
use kanal::Receiver;
use std::collections::{HashMap, VecDeque};

mod color_editor;
mod action_converters;
mod binding_helpers;
mod input_column;
mod learn_capture;
mod monitor_panel;
mod output_column;
mod test_panel;
mod value_editor;
mod value_source_converters;

use binding_helpers::add_matching_output_binding;
use color_editor::render_color_mappings_section;
use input_column::render_input_column;
use learn_capture::apply_learn_captures;
use monitor_panel::show_live_midi_monitor;
use output_column::render_output_column;
use test_panel::{render_test_device_box, sync_runtime_test_state};

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
    test_state_by_controller:
        HashMap<crate::storage::asset_id::AssetId<MidiController>, MidiControllerTestState>,
}

#[derive(Clone)]
#[derive(Default)]
struct MidiControllerTestState {
    selected_output_port: Option<String>,
    value_output_overrides: HashMap<usize, u8>,
    color_overrides_by_mapping_index: HashMap<usize, u8>,
    hovered_status_data1: Option<(u8, u8)>,
}


fn shift_u8_override_indices(map: &mut HashMap<usize, u8>, removed_index: usize) {
    let mut shifted = HashMap::new();
    for (&index, &value) in map.iter() {
        if index == removed_index {
            continue;
        }
        let new_index = if index > removed_index { index - 1 } else { index };
        shifted.insert(new_index, value);
    }
    *map = shifted;
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
            let entry = by_port.entry(event.port_name.clone()).or_insert_with(|| {
                monitor::MidiPortDiagnostics {
                    port_name: event.port_name.clone(),
                    ..Default::default()
                }
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
            runtime::clear_all_test_states();
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

                        match self.tree.selected() {
                            TreeSelection::Asset(controller) => {
                                ui.separator();
                                let test_state = self
                                    .test_state_by_controller
                                    .entry(controller.id)
                                    .or_default();
                                midi_controller_editor(
                                    ui,
                                    controller,
                                    test_state,
                                    &mut self.dirty,
                                    collections,
                                    &mut self.filter,
                                    &diagnostics,
                                    learn_state,
                                );
                            }
                            _ => {
                                for state in self.test_state_by_controller.values_mut() {
                                    state.color_overrides_by_mapping_index.clear();
                                }
                            }
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


fn midi_controller_editor(
    ui: &mut Ui,
    controller: &mut Asset<MidiController>,
    test_state: &mut MidiControllerTestState,
    dirty: &mut bool,
    collections: &mut Collections,
    filter: &mut String,
    diagnostics: &[monitor::MidiPortDiagnostics],
    learn_state: &mut LearnState,
) {
    learn_state.flush_timeouts();
    apply_learn_captures(controller, dirty, collections, learn_state);

    egui::ScrollArea::vertical().show(ui, |ui| {
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
        let test_device_selected = test_state.selected_output_port.is_some();
        let hovered_status_data1 = test_state.hovered_status_data1;
        let mut hovered_status_data1_next = None;

        render_test_device_box(ui, test_state, diagnostics, controller_id);
        ui.add_space(6.0);

        let mut remove_input = None;
        let mut pending_matching_output = Vec::new();
        let mut remove_output = None;

        ui.columns(2, |cols| {
            let (left, right) = cols.split_at_mut(1);
            let (ri, pmo) = render_input_column(
                &mut left[0],
                &mut mapping.input_bindings,
                filter_str,
                hovered_status_data1,
                &mut hovered_status_data1_next,
                dirty,
                controller_id,
                learn_state,
            );
            remove_input = ri;
            pending_matching_output = pmo;
            remove_output = render_output_column(
                &mut right[0],
                &mut mapping.output_bindings,
                &mut mapping.color_mappings,
                test_state,
                test_device_selected,
                filter_str,
                hovered_status_data1,
                &mut hovered_status_data1_next,
                dirty,
                controller_id,
                learn_state,
            );
        });

        test_state.hovered_status_data1 = hovered_status_data1_next;

        ui.separator();
        test_state.color_overrides_by_mapping_index = render_color_mappings_section(
            ui,
            &mut mapping.color_mappings,
            dirty,
            controller_id,
        );

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
            shift_u8_override_indices(&mut test_state.value_output_overrides, index);
            *dirty = true;
        }

        sync_runtime_test_state(controller_id, &mapping.color_mappings, test_state);
    });
}

