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
    pipeline::{Pipeline, RenderDeactivatedScenes},
    project::Project,
    ui::{action::Action, windows::Windows},
    viewport_builder::default_viewport_builder,
};
use egui::{ahash::HashSet, Modifiers, ViewportId};
use std::path::PathBuf;

pub use persistant_state::PersistantState;
pub use svg::{positions, preview_positions, preview_uv, Svg};
pub use timing::Timing;

pub struct App {
    windows: Windows,
    startup: bool,
    output_sender: OutputSender,
    gpu_ready_receiver: GpuReadyReceiver,
    timing: Timing,
    project_path: Option<PathBuf>,
    pipeline: Pipeline,
    other_main_windows: HashSet<ViewportId>,

    blackout: bool,
    svg: Option<Svg>,
    selected_scene_instance: usize,
    hovered_effect: usize,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(_loading) = crate::storage::loading_state() {
            //TODO: Display
        } else if let Some(err) = crate::storage::error_state() {
            println!("Error: {}", err);
            //TODO: Display
        } else if self.startup {
            println!("Loading complete");
            self.startup = false;
        }

        self.timing.tick();
        Input::tick();

        match Action::dequeue() {
            None => (),
            Some(Action::DeleteSelectedSceneInstance) => {
                self.pipeline
                    .remove_scene_instance(self.selected_scene_instance);

                self.selected_scene_instance = self
                    .pipeline
                    .scene_instances()
                    .first()
                    .map(|(index, _scene_instance)| *index)
                    .unwrap_or_default();
                self.pipeline.init_gpu();
            }
            Some(Action::CloneSelectedSceneInstance) => {
                if let Some(scene) = self
                    .pipeline
                    .scene_instance(self.selected_scene_instance)
                    .map(|scene_instance| scene_instance.scene)
                {
                    let index = self.pipeline.add_scene(scene);
                    self.selected_scene_instance = index;
                    self.pipeline.init_gpu();
                }
            }
            Some(Action::InitGPU) => {
                self.pipeline.init_gpu();
            }
            Some(Action::ReloadShaderCode(animation)) => {
                self.pipeline.reload_shader_code(animation);
            }
        }

        if ctx.input_mut(|i| i.consume_key(Modifiers::ALT, egui::Key::Enter)) {
            let mut persistant_state = PersistantState::get();
            persistant_state.fullscreen = !persistant_state.fullscreen;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(
                persistant_state.fullscreen,
            ));
            persistant_state.save();
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
            &self.timing,
            self.blackout,
            if PersistantState::effects_always_render() {
                RenderDeactivatedScenes::Always
            } else {
                RenderDeactivatedScenes::Some(self.selected_scene_instance, self.hovered_effect)
            },
            self.timing.fade_duration(),
        );

        self.draw_main_window(ctx);

        for viewport_id in self.other_main_windows.clone().into_iter() {
            ctx.show_viewport_immediate(
                viewport_id,
                default_viewport_builder()
                    .with_title("Gled: Second Window")
                    .with_inner_size([1300.0, 1024.0])
                    .with_drag_and_drop(true)
                    .with_min_inner_size([300.0, 200.0]),
                |ctx, _viewport_class| {
                    ctx.input(|input| {
                        if input.viewport().close_requested() {
                            self.other_main_windows.remove(&viewport_id);
                        }
                    });

                    self.draw_main_window(ctx);
                },
            );
        }

        self.windows.update(ctx, &self.timing);
        ctx.request_repaint();
    }
}

impl App {
    pub fn draw_main_window(&mut self, ctx: &egui::Context) {
        self.menu(ctx);
        self.config(ctx);
        self.preview(ctx);
        self.effects(ctx);
    }
    pub fn new() -> Option<Self> {
        let (output_sender, gpu_ready_receiver) =
            output_sender::start().expect("Could not start output sender");

        let mut app = Self {
            startup: true,
            output_sender,
            gpu_ready_receiver,
            svg: None,
            project_path: crate::opts::OPTS.project_path.clone(),
            timing: Timing::default(),
            blackout: false,
            selected_scene_instance: 0,
            hovered_effect: 0,
            pipeline: Pipeline::default(),
            windows: Windows::default(),
            other_main_windows: HashSet::default(),
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

        *ExtractOutput::get().routings.lock() = project.output_routings;

        svg::reset();
        self.svg = project.svg;
    }
}
