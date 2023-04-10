mod about;
mod config;
mod menu;
mod svg;
mod timing;

use self::svg::Svg;
use crate::{
    artnet_sender::{self, ArtnetSender},
    get_pipeline,
    logo::logo_image,
    shader_widget::init_shaders,
};
use egui::Image;
use egui_extras::RetainedImage;
use timing::Timing;

pub use svg::{positions, preview_positions};

pub struct App {
    disable_artnet_extraction: bool,
    artnet_ip: String,
    svg: Option<Svg>,
    artnet_sender: ArtnetSender,
    logo_image: RetainedImage,
    timing: Timing,
    about_window_open: bool,
    selected_scene: usize,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Some(fps) = self.timing.framerate() {
            frame.set_window_title(&format!("gled ({fps:.1} fps)"))
        }

        if let Some(svg) = self.svg.as_ref() {
            get_pipeline!(pipeline);
            pipeline.render(
                svg.universes(),
                &mut self.artnet_sender,
                self.timing.beat_progression(),
                self.timing.beats_per_minute,
                self.timing.framerate().unwrap_or_default(),
                self.disable_artnet_extraction,
            );
        }

        self.menu(ctx);
        self.config(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                crate::get_pipeline!(pipeline);

                let size = egui::Vec2::splat(2048.0);
                let res = self
                    .svg
                    .as_ref()
                    .map(|svg| ui.image(svg.image().texture_id(ctx), size));
                let preview = Image::new(pipeline.preview_texture_id(), size);
                match res {
                    Some(res) => {
                        ui.put(res.rect, preview);
                    }
                    None => {
                        ui.add(preview);
                    }
                }

                //egui::Grid::new("scenes").show(ui, |ui| {
                for (_index, scene) in pipeline.scenes() {
                    let res = ui.image(scene.texture_id(), size);
                    if let Some(svg) = self.svg.as_ref() {
                        ui.put(res.rect, Image::new(svg.image().texture_id(ctx), size));
                    }

                    //  ui.end_row();
                }
                // });
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

        //TODO: Empty project
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
            selected_scene: 0,
        })
    }
}
