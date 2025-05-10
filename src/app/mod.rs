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
            Asset,
            project::{
                Project, render_deactivated_scenes::RenderDeactivatedScenes,
                scene_instance_path::SceneInstancePath,
            },
        },
        asset_id::AssetId,
        loading,
    },
    ui::{action::UiAction, viewport_builder::default_viewport_builder, windows::Windows},
};
use eframe::egui_wgpu::Callback;
use egui::{Rect, ViewportId, ahash::HashMap, mutex::Mutex};
use egui_tiles::Tree;
use persistant_state::PersistantState;
use std::{
    sync::{Arc, mpsc::Receiver},
    time::{SystemTime, UNIX_EPOCH},
};
use storage::{show_storage_error, show_storage_loading};
use timing::Timing;

pub struct App {
    pub startup: bool,
    pub windows: Windows,
    pub output_sender: OutputSender,
    pub gpu_ready_receiver: GpuReadyReceiver,
    pub timing: Timing,
    pub project: Option<Project>,
    pub project_id: Option<AssetId<Project>>,
    pub other_main_windows: HashMap<ViewportId, Arc<Mutex<Tree<Pane>>>>,
    pub blackout: bool,
    pub selected_scene_instance: SceneInstancePath,
    pub hovered_scene_instance: SceneInstancePath,
    pub git_commit_message: String,
    pub ui_action_receiver: Receiver<UiAction>,
    pub last_title_update: u64,
    pub midi_output_active: bool,
    pub tree: Arc<Mutex<Tree<Pane>>>,
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
                available_scenes_a: self.project.as_mut().map_or(0, |project| {
                    project
                        .deck(SceneInstancePath::DECK_A)
                        .scenes_instances
                        .len()
                }),
                available_scenes_b: self.project.as_mut().map_or(0, |project| {
                    project
                        .deck(SceneInstancePath::DECK_B)
                        .scenes_instances
                        .len()
                }),
                available_scenes_c: self.project.as_mut().map_or(0, |project| {
                    project
                        .deck(SceneInstancePath::DECK_C)
                        .scenes_instances
                        .len()
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
        let tree = viewport_id
            .and_then(|viewport| self.other_main_windows.get(&viewport).cloned())
            .unwrap_or(self.tree.clone());

        self.menu(ctx, viewport_id);
        self.status_bar(ctx, viewport_id);

        if let Some(error) = crate::storage::error() {
            show_storage_error(ctx, error);
            return;
        } else if let Some(loading) = crate::storage::loading() {
            show_storage_loading(ctx, loading);
            return;
        }

        if self.project.is_some() {
            egui::CentralPanel::default().show(ctx, |ui| {
                tree.lock().ui(self, ui);
            });
        } else {
            self.no_project(ctx);
        }
    }
    pub fn new(ui_action_receiver: Receiver<UiAction>) -> Option<Self> {
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
            git_commit_message: Default::default(),
            ui_action_receiver,
            last_title_update: 0,
            midi_output_active: false,
            tree: new_tree(None, false),
        };

        Some(app)
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

#[derive(Clone, Copy, Debug)]
pub enum Pane {
    Deck(SceneInstancePath),
    Config,
    Preview,
}

impl egui_tiles::Behavior<Pane> for App {
    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: egui_tiles::TileId,
        pane: &mut Pane,
    ) -> egui_tiles::UiResponse {
        match pane {
            Pane::Deck(scene_instance_path) => self.scenes(ui, *scene_instance_path),
            Pane::Config => self.config(ui),
            Pane::Preview => self.preview(ui),
        }
    }

    fn tab_title_for_pane(&mut self, pane: &Pane) -> egui::WidgetText {
        match *pane {
            Pane::Deck(SceneInstancePath::DECK_A) => "Deck A".into(),
            Pane::Deck(SceneInstancePath::DECK_B) => "Deck B".into(),
            Pane::Deck(SceneInstancePath::DECK_C) => "Deck C".into(),
            Pane::Config => "Config".into(),
            Pane::Preview => "Preview".into(),
            _ => unreachable!(),
        }
    }

    fn gap_width(&self, _style: &egui::Style) -> f32 {
        4.0
    }
}

pub fn new_tree(viewport_id: Option<ViewportId>, only_preview: bool) -> Arc<Mutex<Tree<Pane>>> {
    let mut tiles = egui_tiles::Tiles::default();
    let root = if only_preview {
        tiles.insert_pane(Pane::Preview)
    } else {
        let mid_vertical = vec![
            tiles.insert_pane(Pane::Preview),
            tiles.insert_pane(Pane::Config),
        ];
        let horizontal = vec![
            tiles.insert_pane(Pane::Deck(SceneInstancePath::DECK_A)),
            tiles.insert_vertical_tile(mid_vertical),
            tiles.insert_pane(Pane::Deck(SceneInstancePath::DECK_B)),
        ];
        let vertical = vec![
            tiles.insert_horizontal_tile(horizontal),
            tiles.insert_pane(Pane::Deck(SceneInstancePath::DECK_C)),
        ];
        tiles.insert_vertical_tile(vertical)
    };
    Arc::new(Mutex::new(egui_tiles::Tree::new(
        format!("tiles_{viewport_id:?}"),
        root,
        tiles,
    )))
}
