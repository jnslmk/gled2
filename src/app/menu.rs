use crate::artnet_sender;

use super::{svg::Svg, App};
use egui::{Button, Color32, Context, ImageButton, RichText, Ui, Vec2};
use log::{error, info};

impl App {
    pub fn menu(&mut self, ctx: &Context, ui: &mut Ui) {
        ui.style_mut().spacing.interact_size.y = 50.0;
        egui::menu::bar(ui, |ui| {
            let menu_button_size = Vec2::new(100.0, ui.available_height());
            if ui
                .add_sized(
                    menu_button_size,
                    ImageButton::new(
                        self.logo_image.texture_id(ctx),
                        Vec2::splat(ui.available_height()),
                    ),
                )
                .clicked()
            {
                self.about_window_open = true;
            };

            ui.menu_button("File", |ui| {
                if ui.button("Create new project").clicked() {
                    todo!()
                }
                if ui.button("Open project").clicked() {
                    todo!()
                }
                if ui.button("Save project").clicked() {
                    todo!()
                }
                if ui.button("Save project as..").clicked() {
                    todo!()
                }
                if ui.button("Open SVG file").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Open SVG file")
                        .add_filter("svg", &["svg"])
                        .pick_file()
                    {
                        self.svg = match Svg::load(&self.render_state, &path) {
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
            });

            ui.menu_button("Config", |ui| {
                ui.label(RichText::new("frame rate limiter").text_style(egui::TextStyle::Heading));
                ui.add(
                    egui::Slider::new(&mut self.timing.fps_limit, 30..=200)
                        .text("frames per second"),
                );
                ui.separator();
                ui.label(RichText::new("artnet host").text_style(egui::TextStyle::Heading));
                if ui
                    .add(egui::TextEdit::singleline(&mut self.artnet_ip))
                    .changed()
                {
                    if let Ok(ip) = self.artnet_ip.parse() {
                        artnet_sender::set_artnet_ip(ip)
                    }
                };
            });
            ui.separator();

            ui.spacing_mut().slider_width =
                ui.available_width() - (menu_button_size.x * 3.0 + 120.0);
            ui.add(egui::Slider::new(&mut self.timing.beats_per_minute, 1.0..=240.0).text("bpm"));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut blackout = Button::new("Blackout");
                if self.blackout {
                    blackout = blackout.fill(Color32::DARK_RED);
                }
                if ui.add_sized(menu_button_size, blackout).clicked() {
                    self.blackout = !self.blackout;
                }

                self.timing.freeze_button(ui, menu_button_size);
                self.timing.beat_button(ui, menu_button_size);
                ui.separator();
            });
        });
        ui.separator();
    }
}
