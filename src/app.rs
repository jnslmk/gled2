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
    artnet_sender::{self, ArtnetSender, GpuReadyReceiver},
    extract_artnet::ExtractArtnet,
    get_pipeline,
    logo::logo_image,
    shader_widget::init_shaders,
};
use egui::Modifiers;
use egui_extras::RetainedImage;
use timing::Timing;

pub use svg::{positions, preview_positions};

#[derive(Default)]
pub struct App {
    disable_artnet_extraction: bool,
    artnet_ip: String,
    svg: Option<Svg>,
    artnet_sender: Option<ArtnetSender>,
    gpu_ready_receiver: Option<GpuReadyReceiver>,
    logo_image: Option<RetainedImage>,
    timing: Timing,
    about_window_open: bool,
    selected_scene: usize,
    show_preview: bool,
    show_preview_svg: bool,
    show_scenes_svg: bool,
    show_close_dialog: bool,
    allowed_to_close: bool,
    preview_size: f32,
    scene_size: f32,
    main_dimmer: f32,
    fullscreen: bool,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::gui_zoom::zoom_with_keyboard_shortcuts(ctx, frame.info().native_pixels_per_point);

        if ctx.input_mut(|i| i.consume_key(Modifiers::ALT, egui::Key::Enter)) {
            self.fullscreen = !self.fullscreen;
            frame.set_fullscreen(self.fullscreen);
        }

        if let Some(fps) = self.timing.framerate() {
            frame.set_window_title(&format!("gled ({fps:.1} fps)"))
        }

        if let (Some(artnet_sender), Some(gpu_ready_receiver)) = (
            self.artnet_sender.as_mut(),
            self.gpu_ready_receiver.as_mut(),
        ) {
            get_pipeline!(pipeline);
            pipeline.render(
                artnet_sender,
                gpu_ready_receiver,
                self.timing.beat_progression(),
                self.timing.beats_per_minute,
                self.timing.framerate().unwrap_or_default(),
                self.disable_artnet_extraction,
                self.main_dimmer,
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
        let extract_artnet = ExtractArtnet::new();
        let (artnet_sender, gpu_ready_receiver) =
            artnet_sender::start(extract_artnet.clone()).expect("Could not start artnet sender");
        let logo_image = logo_image();

        //TODO: Empty project
        init_shaders(extract_artnet);
        let svg = Svg::load(std::path::Path::new("susifest2022.svg")).ok();

        get_pipeline!(pipeline);
        pipeline.init_gpu();

        Some(Self {
            artnet_ip: "127.0.0.1".to_string(),
            artnet_sender: Some(artnet_sender),
            gpu_ready_receiver: Some(gpu_ready_receiver),
            logo_image: Some(logo_image),
            svg,
            show_preview: true,
            show_preview_svg: true,
            show_scenes_svg: true,
            preview_size: 300.0,
            scene_size: 256.0,
            main_dimmer: 1.0,
            ..Default::default()
        })
    }
}
