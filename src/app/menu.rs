use super::{App, PersistantState, svg::Svg};
use crate::{
    app::timing::{CONNECTED_PEERS, LINK_ACTIVE_COLOR},
    input::artnet::ARTNET_CONFIG,
    pipeline::extract_output::ExtractOutput,
    storage::{STORAGE_DIR, asset::Asset},
    ui::{
        action::UiAction, logo::logo_image, window_common::window_buttons,
        windows::channel_overwrites::ChannelOverwrites,
    },
};
use egui::{Button, Color32, Id, Image, Key, Modifiers, Slider, Stroke, TextFormat, Ui, UiKind, Vec2, ViewportId, text::LayoutJob, KeyboardShortcut};
use log::debug;
use rand::Rng;
use std::{
    sync::{Arc, atomic::Ordering::Relaxed},
    time::{SystemTime, UNIX_EPOCH},
};

impl App {
    pub fn menu(&mut self, ui: &mut Ui, viewport_id: Option<ViewportId>) {
        egui::TopBottomPanel::top(format!("{viewport_id:?} menu")).show_inside(ui, |ui| {
            if crate::storage::loading().is_some() {
                ui.disable();
            }

            egui::MenuBar::new().ui(ui, |ui| {
                let menu_button_size = Vec2::new(100.0, ui.available_height());

                if ui.add(Button::image(Image::new(logo_image()))).clicked() {
                    self.windows.about.open();
                };

                ui.separator();

                let mut open_project = ui
                    .ctx()
                    .input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::O));
                let mut save_project = ui
                    .ctx()
                    .input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::S));
                let mut open_svg_file = ui
                    .ctx()
                    .input_mut(|i| i.consume_key(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::O));
                let mut save_svg_file = ui
                    .ctx()
                    .input_mut(|i| i.consume_key(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::S));

                ui.menu_button("Project", |ui| {
                    ui.set_min_width(300.0);

                    if let Some(project) = self.project_id.and_then(Asset::get) {
                        ui.label(format!("Project: {}", project.name()));
                    } else {
                        ui.label("No project loaded");
                    }

                    ui.separator();

                    if ui
                        .add(Button::new("Load project").shortcut_text(
                            ui.ctx().format_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::O))
                        ))
                        .clicked()
                    {
                        open_project = true;
                        ui.close_kind(UiKind::Menu);
                    }

                    if ui
                        .add_enabled(
                            self.project.is_some(),
                            Button::new("Save project").shortcut_text(
                                ui.ctx().format_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::S))
                            ),
                        )
                        .clicked()
                    {
                        save_project = true;
                        ui.close_kind(UiKind::Menu);
                    }

                    if ui
                        .add_enabled(self.project.is_some(), Button::new("Close project"))
                        .clicked()
                    {
                        self.project.take();
                        ui.close_kind(UiKind::Menu);
                    }

                    ui.separator();

                    if ui
                        .add_enabled(
                            self.project.is_some(),
                            Button::new("Open SVG file").shortcut_text(
                                ui.ctx().format_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::O))
                            ),
                        )
                        .clicked()
                    {
                        open_svg_file = true;
                        ui.close_kind(UiKind::Menu);
                    }

                    if ui
                        .add_enabled(
                            self.svg().is_some(),
                            Button::new("Save SVG file").shortcut_text(
                                ui.ctx().format_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::S))
                            ),
                        )
                        .clicked()
                    {
                        save_svg_file = true;
                        ui.close_kind(UiKind::Menu);
                    }

                    if ui.button("Open SVG templates folder").clicked() {
                        open::that(STORAGE_DIR.join("svg")).ok();
                        ui.close_kind(UiKind::Menu);
                    }

                    ui.separator();

                    if ui
                        .add_enabled(self.project.is_some(), Button::new("External Devices"))
                        .clicked()
                    {
                        self.windows.artnet_input.open();
                        ui.close_kind(UiKind::Menu);
                    }
                    if ui
                        .add_enabled(self.project.is_some(), Button::new("Channel Overwrites"))
                        .clicked()
                    {
                        self.windows.channel_overwrites.open();
                        ui.close_kind(UiKind::Menu);
                    }
                    if ui
                        .add_enabled(self.project.is_some(), Button::new("Output Routings"))
                        .clicked()
                    {
                        self.windows.output_routings.open();
                        ui.close_kind(UiKind::Menu);
                    }
                    if ui
                        .add_enabled(self.project.is_some(), Button::new("Shortcuts"))
                        .clicked()
                    {
                        self.windows.shortcuts.open();
                        ui.close_kind(UiKind::Menu);
                    }
                });

                if open_project {
                    self.windows.projects.open();
                }
                if save_project
                    && let (Some(project), Some(mut asset)) = (
                        self.project.as_ref(),
                        self.project_id
                            .and_then(Asset::get)
                            .map(Arc::unwrap_or_clone),
                    )
                {
                    asset.data = project.to_owned();
                    asset.data.artnet_config = ARTNET_CONFIG.lock().clone();
                    asset.data.output_routings = ExtractOutput::get().routings.lock().clone();
                    asset.data.channel_overwrites = ChannelOverwrites::get();
                    asset.save();
                }

                if open_svg_file {
                    std::thread::spawn(|| {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_title("Open SVG file")
                            .add_filter("svg", &["svg"])
                            .pick_file()
                        {
                            UiAction::SetSvg(match Svg::load(&path) {
                                Ok(svg) => {
                                    debug!("Loaded svg file \"{}\"", path.display());
                                    Some(svg)
                                }
                                Err(err) => {
                                    UiAction::Error(format!(
                                        "Could not load svg file \"{}\": {err:?}",
                                        path.display()
                                    ))
                                    .enqueue();

                                    None
                                }
                            })
                            .enqueue();
                        }
                    });
                }
                if save_svg_file
                    && let (Some(svg), Some(path)) = (
                        self.svg(),
                        rfd::FileDialog::new()
                            .set_title("Save SVG file")
                            .add_filter("svg", &["svg"])
                            .save_file(),
                    )
                {
                    match svg.save(&path) {
                        Ok(_) => {
                            debug!("Saved svg file \"{}\"", path.display());
                        }
                        Err(err) => {
                            UiAction::Error(format!(
                                "Could not save svg file \"{}\": {err:?}",
                                path.display()
                            ))
                            .enqueue();
                        }
                    };
                }

                ui.menu_button("Assets", |ui| {
                    ui.set_min_width(300.0);

                    if ui.button("Animations").clicked() {
                        self.windows.animations.open();
                        ui.close_kind(UiKind::Menu);
                    }
                    if ui.button("Curves").clicked() {
                        self.windows.curves.open();
                        ui.close_kind(UiKind::Menu);
                    }
                    if ui.button("Output Devices").clicked() {
                        self.windows.output_devices.open();
                        ui.close_kind(UiKind::Menu);
                    }
                    if ui.button("Palettes").clicked() {
                        self.windows.palettes.open();
                        ui.close_kind(UiKind::Menu);
                    }
                    if ui.button("Projects").clicked() {
                        self.windows.projects.open();
                        ui.close_kind(UiKind::Menu);
                    }
                    if ui.button("Scenes").clicked() {
                        self.windows.scenes.open();
                        ui.close_kind(UiKind::Menu);
                    }
                });

                ui.menu_button("Config", |ui| {
                    ui.set_min_width(300.0);

                    if ui.button("Git Configuration").clicked() {
                        self.windows.git_config.open();
                        ui.close_kind(UiKind::Menu);
                    }

                    ui.separator();

                    ui.label("GPU preference");
                    let mut prefer_discrete_gpu = PersistantState::prefer_discrete_gpu();
                    if ui
                        .checkbox(
                            &mut prefer_discrete_gpu,
                            "Prefer discrete GPU (changing requires restart of GLED)",
                        )
                        .on_hover_text("Needs restart of gled")
                        .changed()
                    {
                        let mut persistant_state = PersistantState::get();
                        persistant_state.prefer_discrete_gpu = prefer_discrete_gpu;
                        persistant_state.save();
                    }
                    ui.label("Framerate Limiter");
                    let mut fps_limit = PersistantState::fps_limit();
                    ui.spacing_mut().slider_width = 290.0;
                    if ui
                        .add(
                            Slider::new(&mut fps_limit, 30.0..=1000.0)
                                .integer()
                                .custom_formatter(|n, _| format!("{n:.0} fps")),
                        )
                        .changed()
                    {
                        let mut persistant_state = PersistantState::get();
                        persistant_state.fps_limit = fps_limit;
                        persistant_state.save();
                    }
                    let mut persistant_state = PersistantState::get();
                    if ui
                        .checkbox(
                            &mut persistant_state.effects_always_render,
                            "Always render all scenes",
                        )
                        .changed()
                    {
                        persistant_state.save();
                    };

                    ui.separator();

                    egui::gui_zoom::zoom_menu_buttons(ui);

                    ui.separator();

                    let mut persistant_state = PersistantState::get();
                    ui.label("Scene preview size");
                    if ui
                        .add(
                            Slider::new(&mut persistant_state.effects_size, 50.0..=500.0)
                                .show_value(false),
                        )
                        .changed()
                    {
                        persistant_state.save();
                    }
                });

                let mut open_new_window = ui
                    .ctx()
                    .input_mut(|i| i.consume_key(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::N));
                ui.menu_button("Window", |ui| {
                    ui.set_min_width(300.0);

                    if ui
                        .add(Button::new("Open new window").shortcut_text(
                            ui.ctx().format_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::N))
                        ))
                        .clicked()
                    {
                        open_new_window = true;
                        ui.close_kind(UiKind::Menu);
                    }
                });

                if open_new_window {
                    let viewport_id = ViewportId(Id::new(format!(
                        "Second Window {}",
                        rand::rng().random::<u64>()
                    )));
                    self.other_main_windows.insert(viewport_id);
                }

                ui.separator();

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    window_buttons(ui);

                    let underlined = TextFormat {
                        underline: Stroke::new(1.0, Color32::GRAY),
                        ..Default::default()
                    };
                    let mut blackout_text = LayoutJob::default();
                    blackout_text.append("B", 0.0, underlined);
                    blackout_text.append("lackout", 0.0, TextFormat::default());
                    let mut blackout = Button::new(blackout_text);
                    if self.blackout
                        && SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map(|d| d.as_millis() / 200 % 2 == 0)
                            .unwrap_or_default()
                    {
                        blackout = blackout.fill(Color32::DARK_RED);
                    }
                    if ui.add_sized(menu_button_size, blackout).clicked()
                        || self
                            .project
                            .as_ref()
                            .map(|project| project.blackout_input_is_new())
                            .unwrap_or_default()
                    {
                        self.blackout = !self.blackout;
                    }

                    self.blackout_hold = self
                        .project
                        .as_ref()
                        .map(|project| project.blackout_hold_input_is_live())
                        .unwrap_or_default();

                    if CONNECTED_PEERS.load(Relaxed) > 0 {
                        ui.painter().rect_filled(
                            ui.available_rect_before_wrap().shrink2(Vec2::new(0.0, 2.0)),
                            3.0,
                            LINK_ACTIVE_COLOR,
                        );
                    }

                    ui.add_space(5.0);
                    self.timing.tap_button(
                        ui,
                        menu_button_size,
                        self.project
                            .as_ref()
                            .map(|project| project.tap_input_is_new())
                            .unwrap_or_default(),
                    );

                    ui.separator();
                    self.timing.half_button(
                        ui,
                        self.project
                            .as_ref()
                            .map(|project| project.half_input_is_new())
                            .unwrap_or_default(),
                    );
                    self.timing.double_button(
                        ui,
                        self.project
                            .as_ref()
                            .map(|project| project.double_input_is_new())
                            .unwrap_or_default(),
                    );
                    ui.separator();

                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.add_space(5.0);
                        let slider_width = ui.available_width() - 70.0;
                        if slider_width > 5.0 {
                            ui.spacing_mut().slider_width = slider_width;
                        }
                        ui.add(
                            Slider::new(&mut self.timing.change_beats_per_minute, 20.0..=999.0)
                                .custom_formatter(|n, _| format!("{n:.1} bpm")),
                        );
                    });
                });
            });
        });
    }
}
