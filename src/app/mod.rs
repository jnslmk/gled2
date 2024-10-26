mod config;
mod effects;
mod menu;
mod persistant_state;
mod preview;
pub mod svg;
mod timing;

use crate::{
    extract_output::ExtractOutput,
    input::Input,
    output_sender::{self, GpuReadyReceiver, OutputSender},
    pipeline::{Pipeline, RenderDeactivatedEffects},
    project::Project,
    storage::{ChangingAsset, Palette},
    ui::{action::Action, windows::Windows},
};
use egui::Modifiers;
use persistant_state::PersistantState;
use std::path::PathBuf;
use timing::Timing;

pub use svg::{positions, preview_positions, preview_uv, Svg};

pub struct App {
    windows: Windows,
    startup: bool,
    palette: Option<ChangingAsset<Palette>>,
    output_sender: OutputSender,
    gpu_ready_receiver: GpuReadyReceiver,
    timing: Timing,
    persistant_state: PersistantState,
    project_path: Option<PathBuf>,
    pipeline: Pipeline,

    blackout: bool,
    svg: Option<Svg>,
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
        Input::tick();

        match Action::dequeue() {
            None => (),
            Some(Action::DeleteSelectedEffect) => {
                self.pipeline.remove_effect(self.selected_effect);

                self.selected_effect = self
                    .pipeline
                    .effects()
                    .first()
                    .map(|(index, _effect)| *index)
                    .unwrap_or_default();
            }
            Some(Action::CloneSelectedEffect) => {
                if let Some(index) =
                    self.pipeline
                        .effect(self.selected_effect)
                        .cloned()
                        .map(|mut effect| {
                            effect.active = false;
                            self.pipeline.add_effect(effect)
                        })
                {
                    self.selected_effect = index;
                }
            }
            Some(Action::InitGpu) => {
                self.pipeline.init_gpu();
            }
        }

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

        self.windows.update(ctx);
        self.menu(ctx);
        self.config(ctx);
        self.preview(ctx);
        self.effects(ctx);

        ctx.request_repaint();
        self.persistant_state.store();
    }
}

impl App {
    pub fn new() -> Option<Self> {
        let persistant_state = PersistantState::load();
        let (output_sender, gpu_ready_receiver) =
            output_sender::start().expect("Could not start output sender");

        let mut app = Self {
            startup: true,
            output_sender,
            gpu_ready_receiver,
            svg: None,
            project_path: crate::opts::OPTS.project_path.clone(),
            persistant_state,
            timing: Timing::default(),
            blackout: false,
            selected_effect: 0,
            hovered_effect: 0,
            pipeline: Pipeline::default(),
            palette: None,
            windows: Windows::default(),
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
        self.pipeline.init_gpu();

        *ExtractOutput::get().outputs.lock() = project.outputs;

        svg::reset();
        self.svg = project.svg;
    }
}
