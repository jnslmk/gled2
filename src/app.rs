mod about;
mod config;
mod deck_selection;
mod menu;
mod preview;
mod scenes;
mod svg;
mod timing;

use self::svg::Svg;
use crate::{
    artnet_sender::{self, ArtnetSender},
    get_pipeline,
    logo::logo_image,
    shader_widget::init_shaders,
};
use egui_extras::RetainedImage;
use timing::Timing;

pub use svg::{positions, preview_positions};

#[derive(Default)]
pub struct App {
    disable_artnet_extraction: bool,
    artnet_ip: String,
    svg: Option<Svg>,
    artnet_sender: Option<ArtnetSender>,
    logo_image: Option<RetainedImage>,
    timing: Timing,
    about_window_open: bool,
    selected_scene: usize,
    show_preview: bool,
    show_preview_svg: bool,
    show_scenes_svg: bool,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Some(fps) = self.timing.framerate() {
            frame.set_window_title(&format!("gled ({fps:.1} fps)"))
        }

        if let (Some(svg), Some(artnet_sender)) = (self.svg.as_ref(), self.artnet_sender.as_mut()) {
            get_pipeline!(pipeline);
            pipeline.render(
                svg.universes(),
                artnet_sender,
                self.timing.beat_progression(),
                self.timing.beats_per_minute,
                self.timing.framerate().unwrap_or_default(),
                self.disable_artnet_extraction,
            );
        }

        self.about_window(ctx);
        self.menu(ctx);
        self.config(ctx);
        self.preview(ctx);
        self.deck_selection(ctx);
        self.scenes(ctx);
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
            artnet_ip: "127.0.0.1".to_string(),
            artnet_sender: Some(artnet_sender),
            logo_image: Some(logo_image),
            svg,
            show_preview: true,
            show_preview_svg: true,
            show_scenes_svg: true,
            ..Default::default()
        })
    }
}
