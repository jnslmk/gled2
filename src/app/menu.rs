use super::{svg::Svg, App, PersistantState};
use crate::{
    extract_output::ExtractOutput, input::ARTNET_CONFIG, storage::Asset, ui::logo::logo_image,
};
use egui::{
    load::SizedTexture, text::LayoutJob, Button, Color32, Context, Id, ImageButton, Key, Label,
    Modifiers, Rect, RichText, Slider, Stroke, TextFormat, TextStyle, Vec2, ViewportId, WidgetText,
};
use log::{debug, error};
use rand::Rng;
use std::sync::Arc;

impl App {
    pub fn menu(&mut self, ctx: &Context, viewport_id: Option<ViewportId>) {
        egui::TopBottomPanel::top(format!("{viewport_id:?} menu")).show(ctx, |ui| {
            if crate::storage::loading().is_some() {
                ui.set_enabled(false);
            }

            egui::menu::bar(ui, |ui| {
                let menu_button_size = Vec2::new(100.0, ui.available_height());

                if ui
                    .add(ImageButton::new(SizedTexture::new(
                        logo_image().texture_id(ctx),
                        Vec2::splat(ui.available_height()),
                    )))
                    .clicked()
                {
                    self.windows.about.open();
                };

                ui.separator();

                let mut open_project = ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::O));
                let mut save_project = ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::S));
                let mut open_svg_file = ctx
                    .input_mut(|i| i.consume_key(Modifiers::CTRL.plus(Modifiers::SHIFT), Key::O));
                let mut save_svg_file = ctx
                    .input_mut(|i| i.consume_key(Modifiers::CTRL.plus(Modifiers::SHIFT), Key::S));

                ui.menu_button("Project", |ui| {
                    ui.set_min_width(300.0);

                    if let Some(project) = self.project_id.and_then(Asset::get) {
                        ui.label(format!("Project: {}", project.name()));
                    } else {
                        ui.label("No project loaded");
                    }

                    ui.separator();

                    if ui
                        .add(Button::new("Load project").shortcut_text("Ctrl+O"))
                        .clicked()
                    {
                        open_project = true;
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(
                            self.project.is_some(),
                            Button::new("Save project").shortcut_text("Ctrl+S"),
                        )
                        .clicked()
                    {
                        save_project = true;
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui
                        .add_enabled(
                            self.project.is_some(),
                            Button::new("Open SVG file").shortcut_text("Ctrl+Shift+O"),
                        )
                        .clicked()
                    {
                        open_svg_file = true;
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(
                            self.svg().is_some(),
                            Button::new("Save SVG file").shortcut_text("Ctrl+Shift+S"),
                        )
                        .clicked()
                    {
                        save_svg_file = true;
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui
                        .add_enabled(self.project.is_some(), Button::new("Artnet Input"))
                        .clicked()
                    {
                        self.windows.artnet_input.open();
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(self.project.is_some(), Button::new("Output Routings"))
                        .clicked()
                    {
                        self.windows.output_routings.open();
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(self.project.is_some(), Button::new("Shortcuts"))
                        .clicked()
                    {
                        self.windows.shortcuts.open();
                        ui.close_menu();
                    }

                    if let Some(project) = self.project.as_mut() {
                        ui.separator();
                        ui.label("Main Dimmer");
                        ui.spacing_mut().slider_width = 290.0;
                        ui.add(
                            Slider::new(&mut project.main_dimmer, 0.0..=1.0)
                                .custom_formatter(|n, _| format!("{:.0} %", n * 100.0))
                                .custom_parser(|s| s.parse::<f64>().ok().map(|f| f / 100.0)),
                        );
                    }
                });

                if open_project {
                    self.windows.projects.open();
                }
                if save_project {
                    if let (Some(project), Some(mut asset)) = (
                        self.project.as_ref(),
                        self.project_id
                            .and_then(Asset::get)
                            .map(Arc::unwrap_or_clone),
                    ) {
                        asset.data = project.to_owned();
                        asset.data.artnet_config = ARTNET_CONFIG.lock().clone();
                        asset.data.output_routings = ExtractOutput::get().routings.lock().clone();
                        asset.save();
                    }
                }

                if open_svg_file {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Open SVG file")
                        .add_filter("svg", &["svg"])
                        .pick_file()
                    {
                        self.set_svg(match Svg::load(&path) {
                            Ok(svg) => {
                                debug!("Loaded svg file \"{}\"", path.display());
                                Some(svg)
                            }
                            Err(err) => {
                                error!("Could not load svg file \"{}\": {err:?}", path.display());
                                None
                            }
                        });
                    }
                }
                if save_svg_file {
                    if let (Some(svg), Some(path)) = (
                        self.svg(),
                        rfd::FileDialog::new()
                            .set_title("Save SVG file")
                            .add_filter("svg", &["svg"])
                            .save_file(),
                    ) {
                        match svg.save(&path) {
                            Ok(_) => {
                                debug!("Saved svg file \"{}\"", path.display());
                            }
                            Err(err) => {
                                error!("Could not save svg file \"{}\": {err:?}", path.display());
                            }
                        };
                    }
                }

                ui.menu_button("Assets", |ui| {
                    ui.set_min_width(300.0);

                    if ui.button("Animations").clicked() {
                        self.windows.animations.open();
                        ui.close_menu();
                    }
                    if ui.button("Curves").clicked() {
                        self.windows.curves.open();
                        ui.close_menu();
                    }
                    if ui.button("Output Devices").clicked() {
                        self.windows.output_devices.open();
                        ui.close_menu();
                    }
                    if ui.button("Palettes").clicked() {
                        self.windows.palettes.open();
                        ui.close_menu();
                    }
                    if ui.button("Projects").clicked() {
                        self.windows.projects.open();
                        ui.close_menu();
                    }
                    if ui.button("Scenes").clicked() {
                        self.windows.scenes.open();
                        ui.close_menu();
                    }
                });

                ui.menu_button("Config", |ui| {
                    ui.set_min_width(300.0);

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

                    ui.separator();

                    egui::gui_zoom::zoom_menu_buttons(ui);

                    ui.separator();

                    let mut persistant_state = PersistantState::get();
                    ui.label("Size:");
                    if ui
                        .add(
                            Slider::new(&mut persistant_state.effects_size, 50.0..=500.0)
                                .show_value(false),
                        )
                        .changed()
                    {
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
                });

                let mut open_new_window = ctx
                    .input_mut(|i| i.consume_key(Modifiers::CTRL.plus(Modifiers::SHIFT), Key::N));
                let mut open_new_preview_window = ctx
                    .input_mut(|i| i.consume_key(Modifiers::CTRL.plus(Modifiers::SHIFT), Key::P));
                ui.menu_button("Window", |ui| {
                    ui.set_min_width(300.0);

                    if ui
                        .add(Button::new("Open new window").shortcut_text("Ctrl+Shift+N"))
                        .clicked()
                    {
                        open_new_window = true;
                        ui.close_menu();
                    }

                    if ui
                        .add(Button::new("Open new preview window").shortcut_text("Ctrl+Shift+P"))
                        .clicked()
                    {
                        open_new_preview_window = true;
                        ui.close_menu();
                    }

                    ui.separator();

                    let areas = viewport_id
                        .and_then(|viewport| self.other_main_windows.get_mut(&viewport))
                        .unwrap_or(&mut self.areas);
                    let checkbox_with_shortcut =
                        |ui: &mut egui::Ui, check: &mut bool, text: &str, shortcut: &str| {
                            let res = ui.checkbox(check, text);
                            let rect = res.rect;
                            let shortcut = RichText::new(shortcut).color(Color32::DARK_GRAY);
                            let size = WidgetText::from(shortcut.clone())
                                .into_galley(ui, None, ui.available_width(), TextStyle::Body)
                                .rect
                                .size();
                            ui.put(
                                Rect::from_min_max(rect.right_bottom() - size, rect.right_bottom()),
                                Label::new(shortcut).selectable(false),
                            );
                            res.changed()
                        };

                    if checkbox_with_shortcut(ui, &mut areas.fullscreen, "Fullscreen", "Alt+Enter")
                    {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(areas.fullscreen));
                    }
                    if viewport_id.is_some() {
                        checkbox_with_shortcut(ui, &mut areas.menu, "Show Menu", "Ctrl+Shift+1");
                    }
                    checkbox_with_shortcut(ui, &mut areas.preview, "Show Preview", "Ctrl+Shift+2");
                    checkbox_with_shortcut(ui, &mut areas.deck_a, "Show Deck A", "Ctrl+Shift+3");
                    checkbox_with_shortcut(ui, &mut areas.deck_b, "Show Deck B", "Ctrl+Shift+4");
                    checkbox_with_shortcut(ui, &mut areas.deck_c, "Show Deck C", "Ctrl+Shift+5");
                    checkbox_with_shortcut(ui, &mut areas.config, "Show Config", "Ctrl+Shift+6");
                    checkbox_with_shortcut(
                        ui,
                        &mut areas.status_bar,
                        "Show Status Bar",
                        "Ctrl+Shift+7",
                    );

                    ui.separator();

                    if ui.button("Show All").clicked() {
                        areas.menu = true;
                        areas.deck_a = true;
                        areas.deck_b = true;
                        areas.deck_c = true;
                        areas.config = true;
                        areas.preview = true;
                        areas.status_bar = true;
                    }
                    if ui.button("Hide All").clicked() {
                        areas.deck_a = false;
                        areas.deck_b = false;
                        areas.deck_c = false;
                        areas.config = false;
                        areas.preview = false;
                        areas.status_bar = false;
                    }
                });

                if open_new_window {
                    self.other_main_windows.insert(
                        ViewportId(Id::new(format!(
                            "Second Window {}",
                            rand::thread_rng().gen::<u64>()
                        ))),
                        Default::default(),
                    );
                }
                if open_new_preview_window {
                    self.other_main_windows.insert(
                        ViewportId(Id::new(format!(
                            "Preview Window {}",
                            rand::thread_rng().gen::<u64>()
                        ))),
                        super::MainWindowAreas {
                            fullscreen: false,
                            menu: false,
                            deck_a: false,
                            deck_b: false,
                            deck_c: false,
                            preview: true,
                            config: false,
                            status_bar: false,
                        },
                    );
                }

                ui.separator();

                let slider_width = ui.available_width() - (menu_button_size.x * 3.0 + 120.0);
                if slider_width > 5.0 {
                    ui.spacing_mut().slider_width = slider_width;
                }
                ui.add(
                    Slider::new(&mut self.timing.beats_per_minute, 1.0..=240.0)
                        .custom_formatter(|n, _| format!("{:.1} bpm", n)),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let underlined = TextFormat {
                        underline: Stroke::new(1.0, Color32::GRAY),
                        ..Default::default()
                    };
                    let mut blackout_text = LayoutJob::default();
                    blackout_text.append("B", 0.0, underlined);
                    blackout_text.append("lackout", 0.0, TextFormat::default());
                    let mut blackout = Button::new(blackout_text);
                    if self.blackout {
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

                    self.timing.freeze_button(
                        ui,
                        menu_button_size,
                        self.project
                            .as_ref()
                            .map(|project| project.freeze_input_is_new())
                            .unwrap_or_default(),
                    );
                    self.timing.tap_button(
                        ui,
                        menu_button_size,
                        self.project
                            .as_ref()
                            .map(|project| project.tap_input_is_new())
                            .unwrap_or_default(),
                    );
                    ui.separator();
                });
            });
        });
    }
}
