use crate::audio::{AUDIO_DEVICES, audio_device_info_loop, audio_device_labels};
use crate::midi::{monitor, normalize_controller_key};
use crate::storage::asset::project::Project;
use crate::storage::collections::Collections;
use crate::ui::action::UiAction;
use crate::ui::asset::CollectionsChangeButton;
use crate::{
    input::artnet::ARTNET_CONFIG,
    storage::asset::{
        Asset, midi_controller::MidiController, output_device::routing::OutputRouting,
    },
    ui::{
        window_common::{default_viewport_builder, gled_window_frame},
        windows::output_routings::HOVERED_OUTPUT_ROUTING,
    },
};
use chrono::Local;
use egui::{
    Button, CentralPanel, ComboBox, Context, DragValue, Id, Layout, Response, RichText, Slider, Ui,
    Vec2, ViewportId, Widget, WidgetText,
};
use egui_phosphor_icons::icons;
use midir::MidiOutput;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::thread;
use std::thread::JoinHandle;

static REFRESH_DEVICES: AtomicBool = AtomicBool::new(false);

#[derive(Default)]
pub struct ExternalDeviceSettings {
    open: bool,
    selected_submenu: String,
    handle: Option<JoinHandle<()>>,
}

#[derive(Clone)]
struct MidiControllerSelectionRow {
    port_name: String,
    input_connected: bool,
    output_connected: bool,
}

#[derive(Clone)]
struct MidiMappingAssetOptions {
    options: Vec<(
        Option<crate::storage::asset_id::AssetId<MidiController>>,
        String,
    )>,
}

impl ExternalDeviceSettings {
    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn update(
        &mut self,
        ctx: &Context,
        project: &mut Option<Project>,
        collections: &mut Collections,
        midi_diagnostics: &[monitor::MidiPortDiagnostics],
    ) {
        REFRESH_DEVICES.store(self.open && self.selected_submenu == "Audio Input", Relaxed);
        if !self.open {
            return;
        }
        let mut project = if let Some(project) = project {
            project
        } else {
            return;
        };

        ctx.show_viewport_immediate(
            ViewportId(Id::new("external devices settings window")),
            default_viewport_builder()
                .with_inner_size(Vec2::new(800.0, 500.0))
                .with_min_inner_size(Vec2::new(800.0, 500.0))
                .with_resizable(false)
                .with_minimize_button(false)
                .with_maximize_button(true),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.open = false;
                    }
                });

                gled_window_frame(ctx, "External Devices", |ui| {
                    let midi_mapping_rows = collect_midi_controller_rows(project, midi_diagnostics);
                    let midi_mapping_options = collect_midi_mapping_options(collections);
                    let settings_menu = SettingsMenu::new(&mut project, &mut self.selected_submenu)
                        .add_submenu("Audio Input".into(), |ui, project| {
                            audio_input_settings(ui, project)
                        })
                        .add_submenu("MIDI Mappings".into(), |ui, project| {
                            midi_mapping_settings(
                                ui,
                                project,
                                &midi_mapping_rows,
                                &midi_mapping_options,
                            )
                        })
                        .add_submenu("Artnet Bridge".into(), |ui, project| {
                            artnet_bridge_settings(ui, project, collections)
                        })
                        .add_submenu("Artnet Control".into(), |ui, project| {
                            artnet_control_input_settings(ui, project)
                        })
                        .add_submenu("Artnet Trigger".into(), |ui, project| {
                            artnet_trigger_settings(ui, project)
                        });
                    ui.add(settings_menu);
                });

                let is_refreshing = self
                    .handle
                    .as_ref()
                    .map_or_else(|| false, |x| !x.is_finished());
                if !is_refreshing && REFRESH_DEVICES.load(Relaxed) {
                    self.handle = Some(
                        thread::Builder::new()
                            .name("gled:audio:device_refresh".to_string())
                            .spawn(|| audio_device_info_loop(&REFRESH_DEVICES))
                            .expect("Could not spawn audio device refresh thread"),
                    );
                }
            },
        );
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }
}

fn audio_input_settings(ui: &mut Ui, state: &mut Project) {
    let project = state;
    ui.heading("Audio Input Device");
    let mut selected = project.audio_input_device.clone();
    let old_selected = selected.clone();

    let devices = audio_device_labels(&AUDIO_DEVICES.lock().unwrap().clone());
    ui.selectable_value(&mut selected, None, "None");
    for (id, label) in devices {
        ui.selectable_value(&mut selected, Some(id), label);
    }

    if selected != old_selected {
        log::info!("Audio input device changed to {:?}", selected);
        {
            UiAction::SetAudioDevice(selected).enqueue();
        }
    }
}

fn midi_mapping_settings(
    ui: &mut Ui,
    project: &mut Project,
    controllers: &[MidiControllerSelectionRow],
    options: &MidiMappingAssetOptions,
) {
    ui.heading("Active MIDI Mapping by Controller");
    ui.add_space(3.0);

    if controllers.is_empty() {
        ui.label("No MIDI controllers detected yet. Open MIDI Controllers window to inspect incoming ports.");
        return;
    }

    if options.options.len() <= 1 {
        ui.small("No MIDI mapping assets available. You can still remove existing assignments.");
    }

    egui::ScrollArea::vertical()
        .id_salt("external_devices_midi_mapping_list")
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
        .show(ui, |ui| {
            for controller in controllers {
                ui.separator();
                ui.label(&controller.port_name);
                ui.small(format!(
                    "Input: {} | Output: {}",
                    if controller.input_connected {
                        "connected"
                    } else {
                        "missing"
                    },
                    if controller.output_connected {
                        "connected"
                    } else {
                        "missing"
                    }
                ));

                let current = project.midi_active_mappings.get(&controller.port_name);
                let current = current.or_else(|| {
                    let normalized = normalize_controller_key(&controller.port_name);
                    let mut matches = project
                        .midi_active_mappings
                        .iter()
                        .filter(|(key, _)| normalize_controller_key(key) == normalized)
                        .map(|(_, mapping)| mapping);
                    let first = matches.next()?;
                    if matches.next().is_some() {
                        return None;
                    }
                    Some(first)
                });
                let mut selected_index = options
                    .options
                    .iter()
                    .position(|(controller_id, _)| current.is_some_and(|c| c == controller_id))
                    .unwrap_or(0);

                let original_index = selected_index;

                ComboBox::new(format!("midi_mapping_{}", controller.port_name), "")
                    .selected_text(options.options[selected_index].1.clone())
                    .show_ui(ui, |ui| {
                        for (index, (_, label)) in options.options.iter().enumerate() {
                            ui.selectable_value(&mut selected_index, index, label);
                        }
                    });

                let remove_clicked = selected_index != 0 && ui.button("Remove Mapping").clicked();
                if remove_clicked {
                    selected_index = 0;
                }

                // Only update mapping if user explicitly changed the selection or removed mapping
                let combobox_changed = selected_index != original_index;

                if combobox_changed || remove_clicked {
                    let (controller_id, _) = &options.options[selected_index];

                    // Always use exact port name as the key for specificity
                    // This ensures each port can have its own mapping
                    let exact_key = controller.port_name.clone();

                    // Now set/update the exact mapping
                    match controller_id {
                        Some(controller_id) => {
                            project
                                .midi_active_mappings
                                .insert(exact_key, Some(*controller_id));
                        }
                        None => {
                            // Removing mapping: ensure exact key is removed
                            project.midi_active_mappings.remove(&exact_key);
                        }
                    }
                }
            }
        });
}

fn collect_midi_mapping_options(collections: &Collections) -> MidiMappingAssetOptions {
    let mut options = vec![(None, "None".to_owned())];
    let mut controllers: Vec<_> = Asset::<MidiController>::all(collections);
    controllers.sort_by(|a, b| a.name().cmp(b.name()));

    for controller in controllers {
        options.push((Some(controller.id), controller.name().to_owned()));
    }

    MidiMappingAssetOptions { options }
}

fn collect_midi_controller_rows(
    project: &Project,
    diagnostics: &[monitor::MidiPortDiagnostics],
) -> Vec<MidiControllerSelectionRow> {
    let configured_ports = project
        .midi_active_mappings
        .keys()
        .map(|key| normalize_controller_key(key));

    let mut all_ports: BTreeSet<String> = BTreeSet::new();
    for diag in diagnostics {
        all_ports.insert(normalize_controller_key(&diag.port_name));
    }
    for output_port in list_midi_output_ports() {
        all_ports.insert(normalize_controller_key(&output_port));
    }
    for configured in configured_ports {
        all_ports.insert(configured);
    }

    all_ports
        .into_iter()
        .filter(|port_name| !is_gled_midi_port(port_name))
        .map(|port_name| {
            let input_connected = diagnostics.iter().any(|diag| {
                normalize_controller_key(&diag.port_name) == port_name && diag.input_connected
            });
            let output_connected = diagnostics.iter().any(|diag| {
                normalize_controller_key(&diag.port_name) == port_name && diag.output_connected
            });
            MidiControllerSelectionRow {
                port_name,
                input_connected,
                output_connected,
            }
        })
        .collect()
}

fn list_midi_output_ports() -> Vec<String> {
    let Ok(output) = MidiOutput::new("gled_external_devices_scan") else {
        return Vec::new();
    };

    output
        .ports()
        .into_iter()
        .filter_map(|port| output.port_name(&port).ok())
        .filter(|name| !name.trim().is_empty())
        .collect()
}

fn is_gled_midi_port(port_name: &str) -> bool {
    port_name.contains("gled_read_input")
    || port_name.contains("gled_write_output")
}

fn artnet_control_input_settings(ui: &mut Ui, project: &mut Project) {
    project.artnet_control_config(|config| {
        ui.heading("Artnet Control");
        ui.checkbox(&mut config.active, "Active");
        ui.add(DragValue::new(&mut config.universe).range(0..=32767));
    });
}

fn artnet_bridge_settings(ui: &mut Ui, _project: &mut Project, collections: &mut Collections) {
    let mut config = ARTNET_CONFIG.lock();
    ui.heading("Artnet Bridge");
    ui.add_space(3.0);

    if config.bridge.is_empty() {
        ui.label("No artnet data received..");
    }

    egui::ScrollArea::vertical()
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
        .id_salt("artnet_bridge_output_scroll")
        .show(ui, |ui| {
            ui.with_layout(Layout::top_down_justified(egui::Align::Min), |ui| {
                let mut hovered_output_routing = None;

                for (universe, bridge) in config.bridge.iter_mut() {
                    ui.label(RichText::new(format!("Universe: {universe}")).heading());

                    ui.horizontal(|ui| {
                        ui.label(format!("Updates: {}", bridge.updates));
                        ui.label(
                            bridge
                                .last_data_at
                                .with_timezone(&Local)
                                .format("(Last: %H:%M:%S)")
                                .to_string(),
                        );

                        bridge
                            .output_routing
                            .device
                            .collections_change_button(ui, collections);

                        if let Some(device) = bridge
                            .output_routing
                            .device
                            .and_then(|asset_id| Asset::get(asset_id, collections))
                        {
                            let device_universes = device.data.universes();
                            if !device_universes.is_empty() {
                                ComboBox::new(format!("{universe}_universe"), "")
                                    .selected_text(match bridge.output_routing.universe {
                                        None => WidgetText::from("No universe selected"),
                                        Some(universe) => WidgetText::from(universe.to_string()),
                                    })
                                    .width(150.0)
                                    .show_ui(ui, |ui| {
                                        for device_universe in device_universes {
                                            let res = ui.selectable_value(
                                                &mut bridge.output_routing.universe,
                                                Some(*device_universe),
                                                device_universe.to_string(),
                                            );
                                            if res.hovered() {
                                                hovered_output_routing = Some(OutputRouting {
                                                    device: Some(device.id),
                                                    universe: Some(*device_universe),
                                                });
                                            }
                                        }
                                    });
                            }

                            ComboBox::new(format!("{universe}_merge"), "")
                                .selected_text(match bridge.htp {
                                    true => "HTP",
                                    false => "LTP",
                                })
                                .width(50.0)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut bridge.htp, true, "HTP");
                                    ui.selectable_value(&mut bridge.htp, false, "LTP");
                                });

                            if ui.add(Button::new(icons::X)).clicked() {
                                bridge.output_routing = OutputRouting::default();
                            }
                        }
                    });
                }

                *HOVERED_OUTPUT_ROUTING.lock() = hovered_output_routing;
            });
        });
}

fn artnet_trigger_settings(ui: &mut Ui, _project: &mut Project) {
    let mut config = ARTNET_CONFIG.lock();
    ui.heading("Artnet Trigger");
    ui.add_space(3.0);

    ui.separator();
    ui.label("Universe");
    ui.add(Slider::new(&mut config.universe, 0..=32768));
}

struct SettingsMenu<'a, S> {
    selected_submenu: &'a mut String,
    edit_state: &'a mut S,
    submenus: BTreeMap<String, Box<dyn FnOnce(&mut Ui, &mut S) + 'a>>,
}

impl<'a, S> SettingsMenu<'a, S> {
    fn new(edit_state: &'a mut S, selected_submenu: &'a mut String) -> Self {
        Self {
            selected_submenu,
            edit_state,
            submenus: BTreeMap::default(),
        }
    }
    fn add_submenu(
        mut self,
        submenu: String,
        ui_callback: impl FnOnce(&mut Ui, &mut S) + 'a,
    ) -> Self {
        self.submenus.insert(submenu, Box::new(ui_callback));
        self
    }
}

impl<'a, S> Widget for SettingsMenu<'a, S> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        debug_assert!(!self.submenus.is_empty());
        egui::Panel::left("artnet_input_config")
            .resizable(true)
            .size_range(100.0..=200.0)
            .default_size(150.0)
            .show_inside(ui, |ui| {
                ui.take_available_space();
                self.submenus.keys().for_each(|key| {
                    ui.selectable_value(self.selected_submenu, key.clone(), key);
                });
            });
        if !self.submenus.contains_key(self.selected_submenu) {
            *self.selected_submenu = self.submenus.keys().next().unwrap().clone();
        }

        let submenu = self.submenus.remove(self.selected_submenu).unwrap();
        CentralPanel::default()
            .show_inside(ui, |ui| submenu(ui, self.edit_state))
            .response
    }
}
