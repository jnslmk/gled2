mod config;
mod effects;
mod menu;
mod no_project;
mod persistant_state;
mod preview;
mod project_functions;
mod status_bar;
mod storage;
pub mod svg;
mod timing;

use crate::{
    extract_output::ExtractOutput,
    input::{Input, ARTNET_CONFIG},
    output_sender::{self, GpuReadyReceiver, OutputSender},
    storage::{
        loading, Asset, AssetId, GitCredentials, Project, RenderDeactivatedScenes,
        SceneInstancePath,
    },
    ui::{action::Action, windows::Windows},
    viewport_builder::default_viewport_builder,
};
use egui::{ahash::HashMap, Key, Modifiers, SidePanel, TopBottomPanel, ViewportId};
use std::sync::Arc;
use storage::{show_storage_error, show_storage_loading};

pub use persistant_state::PersistantState;
pub use svg::{positions, preview_positions, preview_uv, Svg};
pub use timing::Timing;

pub struct App {
    startup: bool,
    areas: MainWindowAreas,
    windows: Windows,
    output_sender: OutputSender,
    gpu_ready_receiver: GpuReadyReceiver,
    timing: Timing,
    project: Option<Project>,
    project_id: Option<AssetId<Project>>,
    other_main_windows: HashMap<ViewportId, MainWindowAreas>,
    blackout: bool,
    selected_scene_instance: SceneInstancePath,
    hovered_scene_instance: SceneInstancePath,
    git_ui_state: GitUiState,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if loading().is_none() && self.startup {
            self.startup = false;
            if let Some(project) = PersistantState::get().last_project_id {
                Action::SetProject(project).enqueue();
            }
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
                (_, Some(Action::CloseWindow(viewport_id))) => {
                    self.other_main_windows.remove(&viewport_id);
                }
                (_, None) => break,
                (None, _) => (),
            }
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

        self.draw_main_window(ctx, None);

        let viewport_ids = self.other_main_windows.keys().copied().collect::<Vec<_>>();
        for viewport_id in viewport_ids {
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
                            Action::CloseWindow(viewport_id).enqueue();
                        }
                    });

                    self.draw_main_window(ctx, Some(viewport_id));
                },
            );
        }

        self.windows
            .update(ctx, &self.timing, self.project.as_mut());
        ctx.request_repaint();
    }
}

impl App {
    pub fn draw_main_window(&mut self, ctx: &egui::Context, viewport_id: Option<ViewportId>) {
        let areas = {
            let areas = viewport_id
                .and_then(|viewport| self.other_main_windows.get_mut(&viewport))
                .unwrap_or(&mut self.areas);
            if ctx.input_mut(|i| i.consume_key(Modifiers::ALT, Key::Enter)) {
                areas.fullscreen = !areas.fullscreen;
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(areas.fullscreen));
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL.plus(Modifiers::SHIFT), Key::Num1))
                && viewport_id.is_some()
            {
                areas.menu = !areas.menu;
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL.plus(Modifiers::SHIFT), Key::Num2)) {
                areas.preview = !areas.preview;
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL.plus(Modifiers::SHIFT), Key::Num3)) {
                areas.deck_a = !areas.deck_a;
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL.plus(Modifiers::SHIFT), Key::Num4)) {
                areas.deck_b = !areas.deck_b;
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL.plus(Modifiers::SHIFT), Key::Num5)) {
                areas.deck_c = !areas.deck_c;
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL.plus(Modifiers::SHIFT), Key::Num6)) {
                areas.config = !areas.config;
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL.plus(Modifiers::SHIFT), Key::Num7)) {
                areas.status_bar = !areas.status_bar;
            }
            *areas
        };

        if areas.menu {
            self.menu(ctx, viewport_id);
        }

        if areas.status_bar {
            self.status_bar(ctx, viewport_id);
        }

        if let Some(loading) = crate::storage::loading() {
            show_storage_loading(ctx, loading);
            return;
        } else if let Some(error) = crate::storage::error() {
            show_storage_error(ctx, error);
            return;
        }

        if self.project.is_some() {
            if areas.deck_c {
                TopBottomPanel::bottom(format!("{viewport_id:?} deck c"))
                    .resizable(true)
                    .min_height(100.0)
                    .show(ctx, |ui| {
                        self.scenes(ui, SceneInstancePath::DECK_C);
                    });
            }
            if areas.deck_a {
                SidePanel::left(format!("{viewport_id:?} deck a"))
                    .resizable(true)
                    .default_width(250.0)
                    .min_width(100.0)
                    .show(ctx, |ui| {
                        self.scenes(ui, SceneInstancePath::DECK_A);
                    });
            }
            if areas.deck_b {
                SidePanel::right(format!("{viewport_id:?} deck b"))
                    .resizable(true)
                    .default_width(250.0)
                    .min_width(100.0)
                    .show(ctx, |ui| {
                        self.scenes(ui, SceneInstancePath::DECK_B);
                    });
            }

            if areas.config {
                self.config(ctx, viewport_id);
            }
            if areas.preview {
                self.preview(ctx);
            }
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
            areas: Default::default(),
            git_ui_state: Default::default(),
        };

        Some(app)
    }
}

#[derive(Clone, Copy)]
struct MainWindowAreas {
    fullscreen: bool,
    menu: bool,
    deck_a: bool,
    deck_b: bool,
    deck_c: bool,
    preview: bool,
    config: bool,
    status_bar: bool,
}

impl Default for MainWindowAreas {
    fn default() -> Self {
        Self {
            fullscreen: false,
            menu: true,
            deck_a: true,
            deck_b: true,
            deck_c: true,
            preview: true,
            config: true,
            status_bar: true,
        }
    }
}

pub struct GitUiState {
    pub url: String,
    pub commit_message: String,
    pub use_agent: bool,
    pub passphrase: String,
}

impl Default for GitUiState {
    fn default() -> Self {
        let persistant_state = PersistantState::get();

        Self {
            url: persistant_state.git_url,
            commit_message: Default::default(),
            use_agent: matches!(persistant_state.git_credentials, GitCredentials::Agent),
            passphrase: Default::default(),
        }
    }
}
