#![allow(dead_code)]

mod animation;
mod artnet_sender;
mod extract_artnet;
mod icon;
mod logging;
mod mix_artnet;
mod opts;
mod pipeline;
mod scene;
mod shader_widget;
mod svg;
mod texture_to_artnet;

use artnet_sender::ArtnetSender;
use eframe::egui_wgpu::WgpuConfiguration;
use egui::TextureId;
use shader_widget::init_shader;
use svg::{MeasurementPoints, Svg, Universes};

fn main() {
    logging::init();
    artnet_sender::set_artnet_host("192.168.1.255".to_string());

    let options = eframe::NativeOptions {
        drag_and_drop_support: true,
        initial_window_size: Some([1280.0, 1024.0].into()),
        renderer: eframe::Renderer::Wgpu,
        icon_data: Some(icon::icon()),
        wgpu_options: WgpuConfiguration {
            //present_mode: eframe::wgpu::PresentMode::Immediate,
            ..Default::default()
        },
        follow_system_theme: false,
        ..Default::default()
    };
    eframe::run_native(
        "gled",
        options,
        Box::new(|cc| Box::new(Gled::new(cc).unwrap())),
    )
    .expect("Could not run native");
}

struct Gled {
    texture_ids: Vec<TextureId>,
    measurement_points: MeasurementPoints,
    universes: Universes,
    artnet_sender: ArtnetSender,
}

impl eframe::App for Gled {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        shader_widget::render(frame, &self.universes, &mut self.artnet_sender);

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for texture_id in self.texture_ids.iter() {
                            ui.image(*texture_id, egui::Vec2::splat(300.0));
                        }
                    });
                });
        });
        ctx.request_repaint();
    }
}

impl Gled {
    pub fn new<'a>(cc: &'a eframe::CreationContext<'a>) -> Option<Self> {
        let artnet_sender = artnet_sender::start().expect("Could not start artnet sender");

        let measurement_points = MeasurementPoints::from(
            &Svg::read(std::path::Path::new("susifest2022.svg")).expect("Could not read svg file"),
        );
        let universes = measurement_points.universes();

        let texture_ids = init_shader(cc, &measurement_points);
        Some(Self {
            texture_ids,
            measurement_points,
            universes,
            artnet_sender,
        })
    }
}
