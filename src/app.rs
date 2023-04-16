mod about;
mod config;
mod menu;
mod persistant_state;
mod preview;
mod scenes;
mod svg;
mod timing;

use std::path::PathBuf;

use crate::{
    artnet_sender::{self, ArtnetSender, GpuReadyReceiver},
    extract_artnet::ExtractArtnet,
    logo::logo_image,
    pipeline::{Pipeline, RenderDeactivatedScenes},
    project::Project,
};
use egui::Modifiers;
use egui_extras::RetainedImage;
use persistant_state::PersistantState;
use timing::Timing;

pub use svg::{positions, preview_positions, Svg};

pub struct App {
    artnet_sender: ArtnetSender,
    gpu_ready_receiver: GpuReadyReceiver,
    logo_image: RetainedImage,
    extract_artnet: ExtractArtnet,
    timing: Timing,
    persistant_state: PersistantState,
    project_path: Option<PathBuf>,
    pipeline: Pipeline,

    artnet_ip_input: String,
    blackout: bool,
    svg: Option<Svg>,
    about_window_open: bool,
    selected_scene: usize,
    hovered_scene: usize,
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

        frame.set_window_title(&format!(
            "gled - {} {}",
            match self.project_path.as_ref() {
                None => "demo project".to_string(),
                Some(path) => format!("{}", path.display()),
            },
            match self.timing.framerate() {
                Some(fps) => format!("({fps:.1} fps)"),
                None => String::new(),
            }
        ));

        self.pipeline.render(
            &mut self.artnet_sender,
            &mut self.gpu_ready_receiver,
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
        let persistant_state = PersistantState::load();
        persistant_state.set_artnet_ip();

        let extract_artnet = ExtractArtnet::new();
        let (artnet_sender, gpu_ready_receiver) =
            artnet_sender::start(extract_artnet.clone()).expect("Could not start artnet sender");
        let logo_image = logo_image();

        let mut app = Self {
            artnet_sender,
            gpu_ready_receiver,
            logo_image,
            extract_artnet,
            svg: None,
            project_path: crate::opts::OPTS.project_path.clone(),
            artnet_ip_input: persistant_state.artnet_ip.clone(),
            persistant_state,
            timing: Timing::default(),
            blackout: false,
            about_window_open: false,
            selected_scene: 0,
            hovered_scene: 0,
            pipeline: Pipeline::default(),
        };

        app.load_project();

        Some(app)
    }

    pub fn load_project(&mut self) {
        let project = Project::load(self.project_path.as_deref());
        self.use_project(project);
    }

    pub fn use_project(&mut self, project: Project) {
        self.pipeline = project.pipeline;
        self.pipeline
            .set_extract_artnet(self.extract_artnet.clone());
        self.pipeline.init_gpu();

        svg::reset();
        self.svg = project.svg;
    }
}
