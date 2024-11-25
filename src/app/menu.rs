use super::{svg::Svg, App, PersistantState};
use crate::{storage::Asset, ui::logo::logo_image};
use egui::{
    load::SizedTexture, text::LayoutJob, Button, Color32, Context, Id, ImageButton, Key, Modifiers,
    Slider, Stroke, TextFormat, Vec2, ViewportId,
};
use log::{debug, error};
use rand::Rng;
use std::sync::Arc;

impl App {
    pub fn menu(&mut self, ctx: &Context) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
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
                if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL.plus(Modifiers::SHIFT), Key::N))
                {
                    self.other_main_windows.insert(ViewportId(Id::new(format!(
                        "Second Window {}",
                        rand::thread_rng().gen::<u64>()
                    ))));
                }
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
                        .add(Button::new("Open project Window").shortcut_text("Ctrl + O"))
                        .clicked()
                    {
                        open_project = true;
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(
                            self.project.is_some(),
                            Button::new("Save project").shortcut_text("Ctrl + S"),
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
                            Button::new("Open SVG file").shortcut_text("Ctrl + Shift + O"),
                        )
                        .clicked()
                    {
                        open_svg_file = true;
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(
                            self.svg().is_some(),
                            Button::new("Save SVG file").shortcut_text("Ctrl + Shift + S"),
                        )
                        .clicked()
                    {
                        save_svg_file = true;
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("Artnet Input").clicked() {
                        self.windows.artnet_input.open();
                        ui.close_menu();
                    }
                    if ui.button("Output Routings").clicked() {
                        self.windows.output_routings.open();
                        ui.close_menu();
                    }
                    if ui.button("Shortcuts").clicked() {
                        self.windows.shortcuts.open();
                        ui.close_menu();
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

                ui.menu_button("Config", |ui| {
                    ui.set_min_width(300.0);

                    ui.label("Framerate Limiter");
                    let mut fps_limit = PersistantState::fps_limit();
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

                    ui.label("Main Dimmer");

                    let mut main_dimmer = PersistantState::main_dimmer();
                    if ui
                        .add(
                            Slider::new(&mut main_dimmer, 0.0..=1.0)
                                .custom_formatter(|n, _| format!("{:.0} %", n * 100.0))
                                .custom_parser(|s| s.parse::<f64>().ok().map(|f| f / 100.0)),
                        )
                        .changed()
                    {
                        let mut persistant_state = PersistantState::get();
                        persistant_state.main_dimmer = main_dimmer;
                        persistant_state.save();
                    }

                    ui.separator();

                    ui.label("UI Zoom");
                    egui::gui_zoom::zoom_menu_buttons(ui);
                });

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

                ui.separator();

                ui.spacing_mut().slider_width =
                    ui.available_width() - (menu_button_size.x * 3.0 + 120.0);
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
                        || PersistantState::blackout_input_is_new()
                    {
                        self.blackout = !self.blackout;
                    }

                    self.timing.freeze_button(ui, menu_button_size);
                    self.timing.beat_button(ui, menu_button_size);
                    ui.separator();
                });
            });
        });
    }
}
