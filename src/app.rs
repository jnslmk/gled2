use egui::{Image, TextureId};
use egui_extras::RetainedImage;

use crate::{
    artnet_sender::{self, ArtnetSender},
    shader_widget::{self, init_shader},
    svg::{MeasurementPoints, Svg, Universes},
};

pub struct App {
    texture_ids: Vec<TextureId>,
    measurement_points: MeasurementPoints,
    universes: Universes,
    artnet_sender: ArtnetSender,
    svg_image: RetainedImage,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        shader_widget::render(frame, &self.universes, &mut self.artnet_sender);

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for texture_id in self.texture_ids.iter() {
                            let size = egui::Vec2::splat(500.0);
                            let res = ui.image(*texture_id, size);
                            ui.put(res.rect, Image::new(self.svg_image.texture_id(ctx), size));
                        }
                    });
                });
        });
        ctx.request_repaint();
    }
}

impl App {
    pub fn new<'a>(cc: &'a eframe::CreationContext<'a>) -> Option<Self> {
        let artnet_sender = artnet_sender::start().expect("Could not start artnet sender");

        let svg =
            Svg::read(std::path::Path::new("susifest2022.svg")).expect("Could not read svg file");
        let measurement_points = MeasurementPoints::from(&svg);
        let universes = measurement_points.universes();
        let svg_image = svg.render().expect("Could not render svg");

        let texture_ids = init_shader(cc, &measurement_points);

        Some(Self {
            texture_ids,
            measurement_points,
            universes,
            artnet_sender,
            svg_image,
        })
    }
}
