use crate::audio::{AUDIO_DEVICES, audio_device_info_loop};
use crate::storage::asset::project::Project;
use crate::storage::collections::Collections;
use crate::ui::action::UiAction;
use crate::ui::asset::CollectionsChangeButton;
use crate::{
    input::artnet::ARTNET_CONFIG,
    storage::asset::{Asset, output_device::routing::OutputRouting},
    ui::{
        window_common::{default_viewport_builder, gled_window_frame},
        windows::output_routings::HOVERED_OUTPUT_ROUTING,
    },
};
use chrono::Local;
use egui::{
    Button, CentralPanel, ComboBox, Context, DragValue, Id, Layout, Response, RichText, SidePanel,
    Slider, Ui, Vec2, ViewportId, Widget, WidgetText,
};
use egui_phosphor_icons::icons;
use std::collections::BTreeMap;
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

impl ExternalDeviceSettings {
    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn update(
        &mut self,
        ctx: &Context,
        project: &mut Option<Project>,
        collections: &mut Collections,
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
                    let settings_menu = SettingsMenu::new(&mut project, &mut self.selected_submenu)
                        .add_submenu("Audio Input".into(), |ui, project| {
                            audio_input_settings(ui, project)
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

    let devices = AUDIO_DEVICES.lock().unwrap().clone();
    ui.selectable_value(&mut selected, None, "None");
    for (id, desc) in devices {
        ui.selectable_value(&mut selected, Some(id), desc.name());
    }

    if selected != old_selected {
        log::info!("Audio input device changed to {:?}", selected);
        {
            UiAction::SetAudioDevice(selected).enqueue();
        }
    }
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
        SidePanel::left("artnet_input_config")
            .resizable(true)
            .width_range(100.0..=200.0)
            .default_width(150.0)
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
