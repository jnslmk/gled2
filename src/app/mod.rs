mod about;
mod config;
mod effects;
mod menu;
mod persistant_state;
mod preview;
mod svg;
mod timing;

use crate::{
    artnet_receiver::ArtnetEvent,
    extract_output::ExtractOutput,
    hotkey::Gamepad,
    logo::logo_image,
    output_sender::{self, GpuReadyReceiver, OutputSender},
    pipeline::{Pipeline, RenderDeactivatedEffects},
    project::Project,
    storage::{ChangingAsset, Palette},
};
use egui::Modifiers;
use egui_extras::RetainedImage;
use persistant_state::PersistantState;
use std::{collections::HashMap, path::PathBuf, sync::mpsc::Receiver};
use timing::Timing;

pub use svg::{positions, preview_positions, preview_uv, Svg};

pub struct App {
    startup: bool,
    receiver: Receiver<ArtnetEvent>,
    palette: Option<ChangingAsset<Palette>>,
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
    artnet_input_window_open: bool,
    selected_effect: usize,
    hovered_effect: usize,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(loading) = crate::storage::loading_state() {
            println!("Loading: {:.0}%", loading * 100.0);
            //TODO: Display
        } else if let Some(err) = crate::storage::error_state() {
            println!("Error: {}", err);
            //TODO: Display
        } else if self.startup {
            println!("Loading complete");
            self.startup = false;
        }

        self.timing.tick(self.persistant_state.fps_limit);
        self.gamepad.tick(&mut self.receiver);

        if ctx.input_mut(|i| i.consume_key(Modifiers::ALT, egui::Key::Enter)) {
            self.persistant_state.fullscreen = !self.persistant_state.fullscreen;
            self.persistant_state.dirty = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(
                self.persistant_state.fullscreen,
            ));
        }

        ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
            "gled - {} {}",
            match self.project_path.as_ref() {
                None => "demo project".to_string(),
                Some(path) => format!("{}", path.display()),
            },
            match self.timing.framerate() {
                Some(fps) => format!("({fps:.1} fps)"),
                None => String::new(),
            }
        )));

        self.pipeline.render(
            &mut self.output_sender,
            &mut self.gpu_ready_receiver,
            self.timing.beat_progression(),
            self.timing.beats_per_minute,
            self.timing.framerate().unwrap_or_default(),
            self.blackout,
            self.persistant_state.main_dimmer,
            if self.persistant_state.effects.always_render {
                RenderDeactivatedEffects::Always
            } else {
                RenderDeactivatedEffects::Some(self.selected_effect, self.hovered_effect)
            },
            self.timing.fade_duration(),
        );

        self.about_window(ctx);
        self.config_output_window(ctx);
        self.config_artnet_input_window(ctx);
        self.menu(ctx);
        self.config(ctx);
        self.preview(ctx);
        self.effects(ctx);

        ctx.request_repaint();
        self.persistant_state.store();
    }
}

impl App {
    pub fn new(receiver: Receiver<ArtnetEvent>) -> Option<Self> {
        let persistant_state = PersistantState::load();
        let extract_output = ExtractOutput::new();
        let (output_sender, gpu_ready_receiver) =
            output_sender::start(extract_output.clone()).expect("Could not start output sender");
        let logo_image = logo_image();

        let mut app = Self {
            startup: true,
            receiver,
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
            artnet_input_window_open: false,
            selected_effect: 0,
            hovered_effect: 0,
            pipeline: Pipeline::default(),
            gamepad: Gamepad::new(),
            inputs: HashMap::new(),
            palette: None,
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
