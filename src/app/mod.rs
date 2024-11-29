mod config;
mod effects;
mod menu;
mod no_project;
mod persistant_state;
mod preview;
mod project_functions;
mod status_bar;
pub mod svg;
mod timing;

use crate::{
    extract_output::ExtractOutput,
    input::{Input, ARTNET_CONFIG},
    output_sender::{self, GpuReadyReceiver, OutputSender},
    storage::{Asset, AssetId, Project, RenderDeactivatedScenes, SceneInstancePath},
    ui::{action::Action, windows::Windows},
    viewport_builder::default_viewport_builder,
};
use egui::{ahash::HashSet, Modifiers, SidePanel, TopBottomPanel, ViewportId};
use std::sync::Arc;

pub use persistant_state::PersistantState;
pub use svg::{positions, preview_positions, preview_uv, Svg};
pub use timing::Timing;

pub struct App {
    windows: Windows,
    startup: bool,
    output_sender: OutputSender,
    gpu_ready_receiver: GpuReadyReceiver,
    timing: Timing,
    project: Option<Project>,
    project_id: Option<AssetId<Project>>,
    other_main_windows: HashSet<ViewportId>,
    blackout: bool,
    selected_scene_instance: SceneInstancePath,
    hovered_scene_instance: SceneInstancePath,
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

        loop {
            match (&mut self.project, Action::dequeue()) {
                (Some(project), Some(Action::DeleteSelectedSceneInstance)) => {
                    project.remove_scene_instance(&mut self.selected_scene_instance);
                    Action::InitGPU.enqueue();
                }
                (Some(project), Some(Action::CloneSelectedSceneInstance)) => {
                    if let Some(scene) = project
                        .scene_instance(self.selected_scene_instance)
                        .map(|scene_instance| scene_instance.scene)
                    {
                        project
                            .deck(self.selected_scene_instance)
                            .add_scene(&mut self.selected_scene_instance, scene);
                        Action::InitGPU.enqueue();
                    }
                }
                (Some(project), Some(Action::InitGPU)) => {
                    project.init_gpu();
                }
                (Some(project), Some(Action::ReloadShaderCode(animation))) => {
                    project.reload_shader_code(animation);
                }
                (Some(project), Some(Action::SendPositions)) => {
                    project.send_positions();
                }
                (_, Some(Action::SetProject(project))) => {
                    if let Some(project) = Asset::get(project) {
                        let mut persistant_state = PersistantState::get();
                        persistant_state.last_project_id = Some(project.id);
                        persistant_state.save();

                        self.project_id = Some(project.id);
                        let project = Arc::unwrap_or_clone(project).data;
                        *ExtractOutput::get().routings.lock() = project.output_routings.clone();
                        *ARTNET_CONFIG.lock() = project.artnet_config.clone();
                        self.project = Some(project);
                    } else {
                        self.windows.artnet_input.close();
                        self.windows.output_routings.close();
                        self.windows.shortcuts.close();
                        self.project.take();
                        self.project_id.take();
                    };
                    Action::InitGPU.enqueue();
                    svg::reset();
                    self.selected_scene_instance = SceneInstancePath::default();
                    self.hovered_scene_instance = SceneInstancePath::default();
                }
                (_, None) => break,
                (None, _) => (),
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
            match self.project_id.and_then(Asset::get) {
                None => "no project loaded".to_owned(),
                Some(project) => project.name().to_owned(),
            },
            match self.timing.framerate() {
                Some(fps) => format!("({fps:.1} fps)"),
                None => String::new(),
            }
        )));

        if let Some(project) = &mut self.project {
            project.render(
                &mut self.output_sender,
                &mut self.gpu_ready_receiver,
                &self.timing,
                self.blackout,
                if PersistantState::effects_always_render() {
                    RenderDeactivatedScenes::Always
                } else {
                    RenderDeactivatedScenes::Some(
                        self.selected_scene_instance,
                        self.hovered_scene_instance,
                    )
                },
                self.timing.fade_duration(),
            );
        }

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

        self.windows
            .update(ctx, &self.timing, self.project.as_mut());
        ctx.request_repaint();
    }
}

impl App {
    pub fn draw_main_window(&mut self, ctx: &egui::Context) {
        self.menu(ctx);
        self.status_bar(ctx);
        if self.project.is_some() {
            TopBottomPanel::bottom("scenes c")
                .resizable(true)
                .show(ctx, |ui| {
                    self.scenes(ui, SceneInstancePath::DECK_C);
                });
            SidePanel::left("scenes a")
                .resizable(true)
                .default_width(280.0)
                .min_width(280.0)
                .show(ctx, |ui| {
                    self.scenes(ui, SceneInstancePath::DECK_A);
                });
            SidePanel::right("scenes b")
                .resizable(true)
                .default_width(280.0)
                .min_width(280.0)
                .show(ctx, |ui| {
                    self.scenes(ui, SceneInstancePath::DECK_B);
                });

            self.preview(ctx);
            self.config(ctx);
        } else {
            self.no_project(ctx);
        }
    }
    pub fn new() -> Option<Self> {
        let (output_sender, gpu_ready_receiver) =
            output_sender::start().expect("Could not start output sender");

        let app = Self {
            startup: true,
            output_sender,
            gpu_ready_receiver,
            timing: Default::default(),
            blackout: true,
            selected_scene_instance: Default::default(),
            hovered_scene_instance: Default::default(),
            project: Default::default(),
            project_id: Default::default(),
            windows: Default::default(),
            other_main_windows: Default::default(),
        };

        if let Some(project) = PersistantState::get().last_project_id {
            Action::SetProject(project).enqueue();
        }

        Some(app)
    }
}
