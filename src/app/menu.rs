mod output;

use super::{svg::Svg, App};
use crate::project::Project;
use eframe::Frame;
use egui::{
    text::LayoutJob, Button, Color32, Context, ImageButton, Key, Modifiers, RichText, Slider,
    Stroke, TextFormat, Vec2,
};
use log::{debug, error};

impl App {
    pub fn menu(&mut self, ctx: &Context, frame: &Frame) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            ui.style_mut().spacing.interact_size.y = 50.0;
            egui::menu::bar(ui, |ui| {
                let menu_button_size = Vec2::new(100.0, ui.available_height());

                if ui
                    .add(ImageButton::new(
                        self.logo_image.texture_id(ctx),
                        Vec2::splat(ui.available_height()),
                    ))
                    .clicked()
                {
                    self.about_window_open = true;
                };
                ui.separator();

                let mut create_new_project =
                    ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::N));
                let mut open_project = ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::O));
                let mut save_project = ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::S));
                let mut save_project_as = ctx
                    .input_mut(|i| i.consume_key(Modifiers::CTRL.plus(Modifiers::SHIFT), Key::S));
                let mut open_svg_file = ctx
                    .input_mut(|i| i.consume_key(Modifiers::CTRL.plus(Modifiers::SHIFT), Key::O));
                let mut save_svg_file = false;

                ui.menu_button("File", |ui| {
                    ui.set_min_width(300.0);

                    if ui
                        .add(Button::new("Create new project").shortcut_text("Ctrl + N"))
                        .clicked()
                    {
                        create_new_project = true;
                        ui.close_menu();
                    }

                    if ui
                        .add(Button::new("Open project").shortcut_text("Ctrl + O"))
                        .clicked()
                    {
                        open_project = true;
                        ui.close_menu();
                    }

                    if ui
                        .add(Button::new("Save project").shortcut_text("Ctrl + S"))
                        .clicked()
                    {
                        save_project = true;
                        ui.close_menu();
                    }

                    if ui
                        .add(Button::new("Save project as..").shortcut_text("Ctrl + Shift + S"))
                        .clicked()
                    {
                        save_project_as = true;
                        ui.close_menu();
                    }

                    if ui
                        .add(Button::new("Open SVG file").shortcut_text("Ctrl + Shift + O"))
                        .clicked()
                    {
                        open_svg_file = true;
                        ui.close_menu();
                    }

                    if ui
                        .add_enabled(self.svg.is_some(), Button::new("Save SVG file"))
                        .clicked()
                    {
                        save_svg_file = true;
                        ui.close_menu();
                    }
                });

                if create_new_project {
                    self.use_project(Project::default());
                }
                if open_project {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Open Project file")
                        .add_filter("gled2", &["gled2"])
                        .pick_file()
                    {
                        self.project_path = Some(path);
                        self.load_project();
                    }
                }
                if save_project || save_project_as {
                    let project_path = self
                        .project_path
                        .as_ref()
                        .filter(|_| save_project)
                        .cloned()
                        .or_else(|| {
                            rfd::FileDialog::new()
                                .set_title("Save Project file")
                                .add_filter("gled2", &["gled2"])
                                .save_file()
                        });
                    if let Some(mut project_path) = project_path {
                        if project_path
                            .extension()
                            .map(|extension| extension.to_string_lossy())
                            != Some("gled2".into())
                        {
                            project_path.set_extension("gled2");
                        }
                        self.project_path = Some(project_path.clone());
                        Project {
                            svg: self.svg.clone(),
                            pipeline: self.pipeline.clone(),
                            outputs: self
                                .extract_output
                                .outputs
                                .read()
                                .expect("outputs is poisoned")
                                .clone(),
                        }
                        .store(&project_path);
                    }
                }
                if open_svg_file {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Open SVG file")
                        .add_filter("svg", &["svg"])
                        .pick_file()
                    {
                        self.svg = match Svg::load(&path) {
                            Ok(svg) => {
                                debug!("Loaded svg file \"{}\"", path.display());
                                Some(svg)
                            }
                            Err(err) => {
                                error!("Could not load svg file \"{}\": {err:?}", path.display());
                                None
                            }
                        };
                    }
                }
                if save_svg_file {
                    if let (Some(svg), Some(path)) = (
                        self.svg.as_ref(),
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

                    ui.label(RichText::new("Framerate Limiter").heading());
                    if ui
                        .add(
                            Slider::new(&mut self.persistant_state.fps_limit, 30.0..=1000.0)
                                .integer()
                                .custom_formatter(|n, _| format!("{n:.0} fps")),
                        )
                        .changed()
                    {
                        self.persistant_state.dirty = true;
                    }

                    ui.separator();

                    if ui.button(RichText::new("Outputs").heading()).clicked() {
                        self.config_output_window_open = true;
                        ui.close_menu();
                    }

                    ui.separator();

                    ui.label(RichText::new("Main Dimmer").heading());
                    if ui
                        .add(
                            Slider::new(&mut self.persistant_state.main_dimmer, 0.0..=1.0)
                                .custom_formatter(|n, _| format!("{:.0} %", n * 100.0))
                                .custom_parser(|s| s.parse::<f64>().ok().map(|f| f / 100.0)),
                        )
                        .changed()
                    {
                        self.persistant_state.dirty = true;
                    }

                    ui.separator();

                    ui.label(RichText::new("UI Zoom").heading());
                    egui::gui_zoom::zoom_menu_buttons(ui, frame.info().native_pixels_per_point);
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
                        || !ctx.wants_keyboard_input()
                            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::B))
                    {
                        self.blackout = !self.blackout;
                    }

                    self.timing.freeze_button(ctx, ui, menu_button_size);
                    self.timing.beat_button(ctx, ui, menu_button_size);
                    ui.separator();
                });
            });
        });
    }
}
