pub mod config;
pub mod effects;
pub mod menu;
pub mod no_project;
pub mod persistant_state;
pub mod preview;
pub mod project_functions;
pub mod status_bar;
pub mod storage;
pub mod svg;
pub mod timing;

use crate::{
    input::Input,
    midi::state::MidiState,
    pipeline::{
        extract_output::ExtractOutput,
        output_sender::{self, GpuReadyReceiver, OutputSender},
        renderer_callback::RendererCallback,
    },
    storage::{
        asset::{
            project::{
                render_deactivated_scenes::RenderDeactivatedScenes,
                scene_instance_path::SceneInstancePath, Project,
            },
            Asset,
        },
        asset_id::AssetId,
        loading,
    },
    ui::{action::UiAction, viewport_builder::default_viewport_builder, windows::Windows},
};
use eframe::egui_wgpu::Callback;
use egui::{ahash::HashMap, Key, Modifiers, Rect, SidePanel, TopBottomPanel, ViewportId};
use persistant_state::PersistantState;
use std::{
    sync::mpsc::Receiver,
    time::{SystemTime, UNIX_EPOCH},
};
use storage::{show_storage_error, show_storage_loading};
use timing::Timing;

pub struct App {
    pub startup: bool,
    pub areas: MainWindowAreas,
    pub windows: Windows,
    pub output_sender: OutputSender,
    pub gpu_ready_receiver: GpuReadyReceiver,
    pub timing: Timing,
    pub project: Option<Project>,
    pub project_id: Option<AssetId<Project>>,
    pub other_main_windows: HashMap<ViewportId, MainWindowAreas>,
    pub blackout: bool,
    pub selected_scene_instance: SceneInstancePath,
    pub hovered_scene_instance: SceneInstancePath,
    pub git_commit_message: String,
    pub ui_action_receiver: Receiver<UiAction>,
    pub last_title_update: u64,
    pub midi_output_active: bool,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ExtractOutput::trigger_output_sender(&self.output_sender);
        self.gpu_ready_receiver
            .recv()
            .expect("GPU ready sender lost");

        if loading().is_none() && self.startup {
            self.startup = false;
            if let Some(project) = PersistantState::get().last_project_id {
                UiAction::SetProject(project).enqueue();
            }
        }

        self.timing.tick();
        Input::tick();
        self.handle_ui_actions();

        if SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            != self.last_title_update
        {
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
            self.last_title_update = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
        }

        if let Some(project) = &mut self.project {
            project.render(
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

        if self.midi_output_active {
            MidiState {
                blackout: self.blackout,
                beat_flank: self.timing.beat_flank(),
                active_scenes: self
                    .project
                    .as_mut()
                    .map_or_else(Default::default, |project| {
                        project
                            .all_scene_instances()
                            .filter_map(
                                |(path, scene)| if scene.active { Some(path) } else { None },
                            )
                            .collect()
                    }),
            }
            .enqueue();
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
                            UiAction::CloseWindow(viewport_id).enqueue();
                        }
                    });

                    self.draw_main_window(ctx, Some(viewport_id));
                },
            );
        }

        self.windows
            .update(ctx, &self.timing, self.project.as_mut());

        let callback = Callback::new_paint_callback(Rect::ZERO, RendererCallback);
        ctx.debug_painter().add(callback);
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

        if let Some(error) = crate::storage::error() {
            show_storage_error(ctx, error);
            return;
        } else if let Some(loading) = crate::storage::loading() {
            show_storage_loading(ctx, loading);
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
            git_commit_message: Default::default(),
            ui_action_receiver: UiAction::init_queue(),
            last_title_update: 0,
            midi_output_active: false,
        };

        Some(app)
    }
}

#[derive(Clone, Copy)]
pub struct MainWindowAreas {
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
    pub use_passphrase: bool,
    pub passphrase: String,
}

impl Default for GitUiState {
    fn default() -> Self {
        let persistant_state = PersistantState::get();

        Self {
            url: persistant_state.git_url,
            use_passphrase: persistant_state.git_credentials.use_passphrase(),
            passphrase: Default::default(),
        }
    }
}
