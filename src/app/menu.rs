use super::{svg::Svg, App};
use crate::artnet_sender;
use eframe::Frame;
use egui::{
    text::LayoutJob, Button, Color32, Context, ImageButton, Key, Modifiers, RichText, Slider,
    Stroke, TextEdit, TextFormat, Vec2,
};
use log::{error, info};

impl App {
    pub fn menu(&mut self, ctx: &Context, frame: &Frame) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            ui.style_mut().spacing.interact_size.y = 50.0;
            egui::menu::bar(ui, |ui| {
                let menu_button_size = Vec2::new(100.0, ui.available_height());

                if let Some(logo_image) = self.logo_image.as_ref() {
                    if ui
                        .add(ImageButton::new(
                            logo_image.texture_id(ctx),
                            Vec2::splat(ui.available_height()),
                        ))
                        .clicked()
                    {
                        self.about_window_open = true;
                    };
                    ui.separator();
                }

                let mut create_new_project =
                    ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::N));
                let mut open_project = ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::O));
                let mut save_project = ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::S));
                let mut save_project_as = ctx
                    .input_mut(|i| i.consume_key(Modifiers::CTRL.plus(Modifiers::SHIFT), Key::S));
                let mut open_svg_file = ctx
                    .input_mut(|i| i.consume_key(Modifiers::CTRL.plus(Modifiers::SHIFT), Key::O));

                ui.menu_button("File", |ui| {
                    if ui
                        .add(Button::new("Create new project").shortcut_text("Ctrl + N"))
                        .clicked()
                    {
                        create_new_project = true;
                    }

                    if ui
                        .add(Button::new("Open project").shortcut_text("Ctrl + O"))
                        .clicked()
                    {
                        open_project = true;
                    }

                    if ui
                        .add(Button::new("Save project").shortcut_text("Ctrl + S"))
                        .clicked()
                    {
                        save_project = true;
                    }

                    if ui
                        .add(Button::new("Save project as..").shortcut_text("Ctrl + Shift + S"))
                        .clicked()
                    {
                        save_project_as = true;
                    }

                    if ui
                        .add(Button::new("Open SVG file").shortcut_text("Ctrl + Shift + O"))
                        .clicked()
                    {
                        open_svg_file = true;
                    }
                });

                if create_new_project {
                    todo!();
                }
                if open_project {
                    todo!();
                }
                if save_project {
                    todo!();
                }
                if save_project_as {
                    todo!();
                }
                if open_svg_file {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Open SVG file")
                        .add_filter("svg", &["svg"])
                        .pick_file()
                    {
                        self.svg = match Svg::load(&path) {
                            Ok(svg) => {
                                info!("Loaded svg file \"{}\"", path.display());
                                Some(svg)
                            }
                            Err(err) => {
                                error!("Could not load svg file \"{}\": {err:?}", path.display());
                                None
                            }
                        };
                    }
                }

                ui.menu_button("Config", |ui| {
                    ui.label(RichText::new("Framerate Limiter").heading());
                    ui.add(
                        Slider::new(&mut self.timing.fps_limit, 30..=1000)
                            .custom_formatter(|n, _| format!("{n} fps")),
                    );

                    ui.separator();

                    ui.label(RichText::new("Artnet IP").heading());
                    if ui.add(TextEdit::singleline(&mut self.artnet_ip)).changed() {
                        if let Ok(ip) = self.artnet_ip.parse() {
                            artnet_sender::set_artnet_ip(ip)
                        }
                    };

                    ui.separator();

                    ui.label(RichText::new("Main Dimmer").heading());
                    ui.add(
                        Slider::new(&mut self.main_dimmer, 0.0..=1.0)
                            .custom_formatter(|n, _| format!("{:.0} %", n * 100.0))
                            .custom_parser(|s| s.parse::<f64>().ok().map(|f| f / 100.0)),
                    );

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
                    if self.disable_artnet_extraction {
                        blackout = blackout.fill(Color32::DARK_RED);
                    }
                    if ui.add_sized(menu_button_size, blackout).clicked()
                        || !ctx.wants_keyboard_input()
                            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::B))
                    {
                        self.disable_artnet_extraction = !self.disable_artnet_extraction;
                    }

                    self.timing.freeze_button(ctx, ui, menu_button_size);
                    self.timing.beat_button(ctx, ui, menu_button_size);
                    ui.separator();
                });
            });
        });
    }
}
