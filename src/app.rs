mod about;
mod config;
mod menu;
mod persistant_state;
mod preview;
mod scenes;
mod svg;
mod timing;

use self::{persistant_state::PersistantState, svg::Svg};
use crate::{
    artnet_sender::{self, ArtnetSender, GpuReadyReceiver},
    extract_artnet::ExtractArtnet,
    get_pipeline,
    logo::logo_image,
    pipeline::RenderDeactivatedScenes,
    shader_widget::init_shaders,
};
use egui::Modifiers;
use egui_extras::RetainedImage;
use timing::Timing;

pub use svg::{positions, preview_positions};

#[derive(Default)]
pub struct App {
    artnet_ip_input: String,
    blackout: bool,
    svg: Option<Svg>,
    artnet_sender: Option<ArtnetSender>,
    gpu_ready_receiver: Option<GpuReadyReceiver>,
    logo_image: Option<RetainedImage>,
    timing: Timing,
    about_window_open: bool,
    selected_scene: usize,
    hovered_scene: usize,

    persistant_state: PersistantState,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.timing.tick(self.persistant_state.fps_limit);

        egui::gui_zoom::zoom_with_keyboard_shortcuts(ctx, frame.info().native_pixels_per_point);
        if ctx.input_mut(|i| i.consume_key(Modifiers::ALT, egui::Key::Enter)) {
            self.persistant_state.fullscreen = !self.persistant_state.fullscreen;
            self.persistant_state.dirty = true;
            frame.set_fullscreen(self.persistant_state.fullscreen);
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
                self.blackout,
                self.persistant_state.main_dimmer,
                if self.persistant_state.background.always_render {
                    RenderDeactivatedScenes::Always
                } else {
                    RenderDeactivatedScenes::Some(self.selected_scene, self.hovered_scene)
                },
                if self.persistant_state.foreground.always_render {
                    RenderDeactivatedScenes::Always
                } else {
                    RenderDeactivatedScenes::Some(self.selected_scene, self.hovered_scene)
                },
            );
        }

        self.about_window(ctx);
        self.menu(ctx, frame);
        self.config(ctx);
        self.preview(ctx);
        self.scenes(ctx);

        ctx.request_repaint();
        self.persistant_state.store();
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
        let persistant_state = PersistantState::load();
        persistant_state.set_artnet_ip();

        Some(Self {
            artnet_sender: Some(artnet_sender),
            gpu_ready_receiver: Some(gpu_ready_receiver),
            logo_image: Some(logo_image),
            svg,
            artnet_ip_input: persistant_state.artnet_ip.clone(),
            persistant_state,
            ..Default::default()
        })
    }
}
