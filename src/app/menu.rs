use super::{App, svg::Svg};
use crate::{
    app::timing::{CONNECTED_PEERS, LINK_ACTIVE_COLOR},
    input::artnet::ARTNET_CONFIG,
    storage::{STORAGE_DIR, asset::Asset},
    ui::{
        action::UiAction, logo::logo_image, window_common::window_buttons,
        windows::channel_overwrites::ChannelOverwrites,
    },
};
use egui::{Button, Color32, Id, Image, Key, KeyboardShortcut, Modifiers, Slider, Stroke, TextFormat, Ui, UiKind, Vec2, ViewportId, text::LayoutJob, ViewportCommand, PointerButton, Sense, Frame};
use log::debug;
use std::{
    sync::{Arc, atomic::Ordering::Relaxed},
    time::{SystemTime, UNIX_EPOCH},
};
use epaint::{RectShape, StrokeKind};

impl App {
    pub fn menu(&mut self, ui: &mut Ui, viewport_id: Option<ViewportId>) {
        egui::Panel::top(format!("{viewport_id:?} menu")).show_inside(ui, |ui| {
            if crate::storage::is_loading() {
                ui.disable();
            }
            let title_bar_response = ui.interact(
                ui.available_rect_before_wrap(),
                Id::new("title_bar"),
                Sense::click_and_drag(),
            );

            // Interact with the title bar (drag to move window):
            if title_bar_response.double_clicked() {
                let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                ui.ctx()
                    .send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
            }

            if title_bar_response.drag_started_by(PointerButton::Primary) {
                ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
                    }

            egui::MenuBar::new().ui(ui, |ui| {
                Frame::new()
                    .show(ui, |ui| {
                        ui.set_height(32.0);

                        ui.columns_const(|[menu_col, bpm_col, window_col]| {
                            self.menus(menu_col);
                            self.bpm_settings(bpm_col);
                            self.window_buttons(window_col);
                        });
                });
            });
        });
    }

    fn menus(&mut self, ui: &mut Ui) {
        ui.horizontal_centered(|ui| {
            ui.take_available_height();

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
            let mut open_svg_file = ui.ctx().input_mut(|i| {
                i.consume_key(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::O)
            });
            let mut save_svg_file = ui.ctx().input_mut(|i| {
                i.consume_key(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::S)
            });

            ui.horizontal(|ui| {
                ui.menu_button("Project", |ui| {
                    ui.set_min_width(300.0);

                    if let Some(project) = self
                        .project_id
                        .and_then(|id| Asset::get(id, &self.collections))
                    {
                        ui.label(format!("Project: {}", project.name()));
                    } else {
                        ui.label("No project loaded");
                    }

                    ui.separator();

                    if ui
                        .add(Button::new("Load project").shortcut_text(
                            ui.ctx().format_shortcut(&KeyboardShortcut::new(
                                Modifiers::COMMAND,
                                Key::O,
                            )),
                        ))
                        .clicked()
                    {
                        open_project = true;
                        ui.close_kind(UiKind::Menu);
                    }

                    if ui
                        .add_enabled(
                            self.project.is_some(),
                            Button::new("Save project").shortcut_text(ui.ctx().format_shortcut(
                                &KeyboardShortcut::new(Modifiers::COMMAND, Key::S),
                            )),
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
                            Button::new("Open SVG file").shortcut_text(ui.ctx().format_shortcut(
                                &KeyboardShortcut::new(
                                    Modifiers::COMMAND | Modifiers::SHIFT,
                                    Key::O,
                                ),
                            )),
                        )
                        .clicked()
                    {
                        open_svg_file = true;
                        ui.close_kind(UiKind::Menu);
                    }

                    if ui
                        .add_enabled(
                            self.svg().is_some(),
                            Button::new("Save SVG file").shortcut_text(ui.ctx().format_shortcut(
                                &KeyboardShortcut::new(
                                    Modifiers::COMMAND | Modifiers::SHIFT,
                                    Key::S,
                                ),
                            )),
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
                        self.windows.external_device_settings.open();
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
                        .and_then(|id| Asset::get(id, &self.collections))
                        .map(Arc::unwrap_or_clone),
                    )
                {
                    asset.data = project.to_owned();
                    asset.data.artnet_config = ARTNET_CONFIG.lock().clone();
                    asset.data.output_routings = self.extract_output.routings.clone();
                    asset.data.channel_overwrites = ChannelOverwrites::get();
                    asset.save(&mut self.collections);
                }

                if open_svg_file {
                    std::thread::Builder::new()
                        .name("gled:ui:open_svg".to_string())
                        .spawn(|| {
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
                        })
                        .ok();
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
                    if ui.button("MIDI Controllers").clicked() {
                        self.windows.midi_controllers.open();
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
                    let mut prefer_discrete_gpu = self.persistant_state.prefer_discrete_gpu();
                    if ui
                        .checkbox(
                            &mut prefer_discrete_gpu,
                            "Prefer discrete GPU (changing requires restart of GLED)",
                        )
                        .on_hover_text("Needs restart of gled")
                        .changed()
                    {
                        self.persistant_state
                            .set_prefer_discrete_gpu(prefer_discrete_gpu);
                        self.persistant_state.save();
                    }
                    ui.label("Framerate Limiter");
                    let mut fps_limit = self.persistant_state.fps_limit();
                    ui.spacing_mut().slider_width = 290.0;
                    if ui
                        .add(
                            Slider::new(&mut fps_limit, 30.0..=1000.0)
                                .integer()
                                .custom_formatter(|n, _| format!("{n:.0} fps")),
                        )
                        .changed()
                    {
                        self.persistant_state.set_fps_limit(fps_limit);
                        self.persistant_state.save();
                    }
                    if ui
                        .checkbox(
                            self.persistant_state.effects_always_render_mut(),
                            "Always render all scenes",
                        )
                        .changed()
                    {
                        self.persistant_state.save();
                    };

                    ui.separator();

                    egui::gui_zoom::zoom_menu_buttons(ui);
                });

                let mut open_new_window = ui.ctx().input_mut(|i| {
                    i.consume_key(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::N)
                });
                ui.menu_button("Window", |ui| {
                    ui.set_min_width(300.0);

                    if ui
                        .add(Button::new("Open new window").shortcut_text(
                            ui.ctx().format_shortcut(&KeyboardShortcut::new(
                                Modifiers::COMMAND.plus(Modifiers::SHIFT),
                                Key::N,
                            )),
                        ))
                        .clicked()
                    {
                        open_new_window = true;
                        ui.close_kind(UiKind::Menu);
                    }
                });

                if open_new_window {
                    let viewport_id =
                        ViewportId(Id::new(format!("Second Window {}", rand::random::<u64>())));
                    self.other_main_windows.insert(viewport_id);
                }
            });
        });
    }

    fn  window_buttons(&mut self, ui: &mut Ui) {
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
            if ui.add_sized(Vec2::new(100.0, 20.0), blackout).clicked()
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

        });
    }

    fn bpm_settings(&mut self, ui: &mut Ui) {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            let rect = ui.available_rect_before_wrap();
            ui.painter().vline(
                rect.left(),
                rect.top()..=rect.bottom(),
                ui.style().visuals.widgets.noninteractive.bg_stroke
            );
            if CONNECTED_PEERS.load(Relaxed) > 0 {
                ui.painter().rect_stroke(
                    ui.available_rect_before_wrap(),
                    0.0,
                    Stroke::new(1.0, LINK_ACTIVE_COLOR),
                    StrokeKind::Outside,
                );
            }
            Frame::new()
                .fill(Color32::from_gray(20))
                .inner_margin(3.0)
                .show(ui, |ui| {
                    ui.take_available_space();

                    // add background shadow
                    ui.painter().add(
                        RectShape::filled(ui.available_rect_before_wrap().expand(1.0),
                                          ui.ctx().global_style().visuals.widgets.active.corner_radius,
                                          Color32::from_white_alpha(10))
                            .with_blur_width(15.0),
                    );

                    let slider_width = 300.0;
                    ui.spacing_mut().slider_width = slider_width;
                    ui.add(
                        Slider::new(&mut self.timing.change_beats_per_minute, 20.0..=999.0)
                            .custom_formatter(|n, _| format!("{n:.1} bpm")),
                    );

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
                    self.timing.tap_button(
                        ui,
                        Vec2::new(50.0, 20.0),
                        self.project
                            .as_ref()
                            .map(|project| project.tap_input_is_new())
                            .unwrap_or_default(),
                    );

                });
            ui.painter().vline(
                rect.right(),
                rect.top()..=rect.bottom(),
                ui.style().visuals.widgets.noninteractive.bg_stroke
            );
        });
    }
}