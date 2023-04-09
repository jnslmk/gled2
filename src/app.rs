mod about;
mod menu;
mod svg;
mod timing;

use self::svg::Svg;
use crate::{
    artnet_sender::{self, ArtnetSender},
    logo::logo_image,
    pipeline::Pipeline,
    shader_widget::{self, init_shaders},
    wgpu_render_state,
};
use egui::Image;
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

            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        crate::get_pipeline!(pipeline);

                        for scene in pipeline.scenes() {
                            let size = egui::Vec2::splat(500.0);
                            let res = ui.image(scene.texture_id(), size);
                            if let Some(svg) = self.svg.as_ref() {
                                ui.put(res.rect, Image::new(svg.image().texture_id(ctx), size));
                            }
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
