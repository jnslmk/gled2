mod about;
mod menu;
mod svg;
mod timing;

use self::svg::Svg;
use crate::{
    animation::Color,
    artnet_sender::{self, ArtnetSender},
    get_pipeline,
    logo::logo_image,
    pipeline::Pipeline,
    shader_widget::{self, init_shaders},
    wgpu_render_state,
};
use egui::{Button, Image, Rect, Vec2};
use egui_extras::RetainedImage;
use timing::Timing;

pub use svg::positions;

pub struct App {
    disable_artnet_extraction: bool,
    artnet_ip: String,
    svg: Option<Svg>,
    artnet_sender: ArtnetSender,
    logo_image: RetainedImage,
    timing: Timing,
    about_window_open: bool,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Some(fps) = self.timing.framerate() {
            frame.set_window_title(&format!("gled ({fps:.1} fps)"))
        }

        if let Some(svg) = self.svg.as_ref() {
            shader_widget::render(
                svg.universes(),
                &mut self.artnet_sender,
                self.timing.beat_progression(),
                self.timing.beats_per_minute,
                self.timing.framerate().unwrap_or_default(),
                self.disable_artnet_extraction,
            );
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            self.menu(ctx, ui);

            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("scenes").show(ui, |ui| {
                    crate::get_pipeline!(pipeline);
                    for (_index, scene) in pipeline.scenes() {
                        let rects = scene
                            .palette
                            .colors()
                            .iter_mut()
                            .map(|color| {
                                let res = ui.color_edit_button_rgb(color.rgb_mut());
                                Rect::from_min_max(res.rect.center(), res.rect.right_bottom())
                            })
                            .collect::<Vec<_>>();
                        for (index, rect) in rects.into_iter().enumerate() {
                            ui.scope(|ui| {
                                ui.style_mut().spacing.interact_size.y = 10.0;
                                if ui.put(rect, Button::new("-")).clicked() {
                                    scene.palette.remove_color(index);
                                }
                            });
                        }

                        if scene.palette.colors.len() < 16
                            && ui
                                .add_sized(
                                    Vec2::new(
                                        ui.style().spacing.interact_size.y,
                                        ui.style().spacing.interact_size.y,
                                    ),
                                    Button::new("+"),
                                )
                                .clicked()
                        {
                            scene.palette.add_color(Color::default());
                        }

                        let size = egui::Vec2::splat(500.0);
                        let res = ui.image(scene.texture_id(), size);
                        if let Some(svg) = self.svg.as_ref() {
                            ui.put(res.rect, Image::new(svg.image().texture_id(ctx), size));
                        }

                        ui.end_row();
                    }
                });
            });
        });

        self.about_window(ctx);
        ctx.request_repaint();

        self.timing.calculate();
    }
}

impl App {
    pub fn new() -> Option<Self> {
        let artnet_sender = artnet_sender::start().expect("Could not start artnet sender");
        let logo_image = logo_image();
        init_shaders();
        let svg = Svg::load(std::path::Path::new("susifest2022.svg")).ok();

        get_pipeline!(pipeline);
        pipeline.init_gpu();

        Some(Self {
            disable_artnet_extraction: false,
            artnet_ip: "127.0.0.1".to_string(),
            svg,
            artnet_sender,
            logo_image,
            timing: Default::default(),
            about_window_open: false,
        })
    }
}
