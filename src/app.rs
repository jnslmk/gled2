mod about;
mod config;
mod menu;
mod persistant_state;
mod preview;
mod scenes;
mod svg;
mod timing;

use crate::{
    extract_output::ExtractOutput,
    hotkey::Gamepad,
    logo::logo_image,
    output_sender::{self, GpuReadyReceiver, OutputSender},
    pipeline::{Pipeline, RenderDeactivatedScenes},
    project::Project,
};
use egui::Modifiers;
use egui_extras::RetainedImage;
use persistant_state::PersistantState;
use std::{collections::HashMap, path::PathBuf};
use timing::Timing;

pub use svg::{positions, preview_positions, preview_uv, Svg};

pub struct App {
    output_sender: OutputSender,
    gpu_ready_receiver: GpuReadyReceiver,
    logo_image: RetainedImage,
    extract_output: ExtractOutput,
    timing: Timing,
    persistant_state: PersistantState,
    project_path: Option<PathBuf>,
    pipeline: Pipeline,
    gamepad: Gamepad,
    inputs: HashMap<String, String>,

    blackout: bool,
    svg: Option<Svg>,
    about_window_open: bool,
    config_output_window_open: bool,
    selected_scene: usize,
    hovered_scene: usize,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.timing.tick(self.persistant_state.fps_limit);
        self.gamepad.tick();

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
            &mut self.output_sender,
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
            self.timing.fade_duration(),
        );

        self.about_window(ctx);
        self.config_output_window(ctx);
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
        let extract_output = ExtractOutput::new();
        let (output_sender, gpu_ready_receiver) =
            output_sender::start(extract_output.clone()).expect("Could not start output sender");
        let logo_image = logo_image();

        let mut app = Self {
            output_sender,
            gpu_ready_receiver,
            logo_image,
            extract_output,
            svg: None,
            project_path: crate::opts::OPTS.project_path.clone(),
            persistant_state,
            timing: Timing::default(),
            blackout: false,
            about_window_open: false,
            config_output_window_open: false,
            selected_scene: 0,
            hovered_scene: 0,
            pipeline: Pipeline::default(),
            gamepad: Gamepad::new(),
            inputs: HashMap::new(),
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
            .set_extract_output(self.extract_output.clone());
        self.pipeline.init_gpu();

        *self
            .extract_output
            .outputs
            .write()
            .expect("outputs is poisoned") = project.outputs;

        svg::reset();
        self.svg = project.svg;
    }
}
