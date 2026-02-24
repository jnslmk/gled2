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

use crate::storage::asset::scene::grid::GridLocation;
use crate::storage::collections::Collections;
use crate::{
    input::Input,
    midi::state::MidiState,
    pipeline::renderer_callback::RendererCallback,
    storage::{
        asset::{Asset, palette::Palette, project::Project},
        asset_id::AssetId,
        loading,
    },
    ui::{
        action::UiAction, asset_tree::AssetTree, window_common::default_viewport_builder,
        windows::Windows,
    },
};
use eframe::egui_wgpu::Callback;
use egui::{CentralPanel, Id, Rect, UiBuilder, ViewportId, ahash::HashSet};
use persistant_state::PersistantState;
use std::{sync::mpsc::Receiver, time::Instant};
use storage::{show_storage_error, show_storage_loading};
use timing::Timing;

pub struct App {
    pub startup: bool,
    pub windows: Windows,
    pub timing: Timing,
    pub last_always_render_fps_frame: Instant,
    pub project: Option<Project>,
    pub project_id: Option<AssetId<Project>>,
    pub other_main_windows: HashSet<ViewportId>,
    pub blackout: bool,
    pub blackout_hold: bool,
    pub selected_scene_instance: GridLocation,
    pub git_commit_message: String,
    pub ui_action_receiver: Receiver<UiAction>,
    pub last_title: String,
    pub midi_output_active: bool,
    pub palette_asset_tree: AssetTree<Palette>,
    pub palette_asset_tree_id: Option<Id>,
    pub collections: Collections,
}

impl eframe::App for App {
    #[cfg_attr(feature = "profiling", profiling::function)]
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(feature = "profiling")]
        {
            crate::WGPU_PROFILER
                .lock()
                .end_frame()
                .expect("Could not end WGPU profiler frame");
            let latest_profiler_results = crate::WGPU_PROFILER
                .lock()
                .process_finished_frame(crate::wgpu_render_state().queue.get_timestamp_period());
            wgpu_profiler::puffin::output_frame_to_puffin(
                &mut crate::PUFFIN_GPU_PROFILER.lock(),
                latest_profiler_results.as_deref().unwrap_or_default(),
            );
            crate::PUFFIN_GPU_PROFILER.lock().new_frame();
        }

        self.collections.update();

        if loading().is_none() && self.startup {
            self.startup = false;
            if let Some(project) = PersistantState::get().last_project_id {
                UiAction::SetProject(project).enqueue();
            }
        }

        self.timing.tick();
        Input::tick();
        self.handle_ui_actions();

        let title = format!(
            "gled - {}",
            match self
                .project_id
                .and_then(|id| Asset::get(id, &self.collections))
            {
                None => "No project".to_string(),
                Some(project) => project.name().to_string(),
            }
        );
        if title != self.last_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }

        if let Some(project) = &mut self.project {
            project.render(
                &self.timing,
                self.blackout || self.blackout_hold,
                if PersistantState::effects_always_render() && {
                    self.last_always_render_fps_frame.elapsed().as_secs_f32() > 1.0 / 30.0
                } {
                    self.last_always_render_fps_frame = Instant::now();
                    true
                } else {
                    false
                },
                self.timing.fade_duration(),
                &self.collections,
            );
        }

        if self.midi_output_active {
            MidiState {
                blackout: self.blackout || self.blackout_hold,
                beat_flank: self.timing.beat_flank(),
                active_scenes: self
                    .project
                    .as_mut()
                    .map_or_else(Default::default, |project| {
                        project
                            .scenes_instances_grid
                            .iter_mut()
                            .filter_map(
                                |(location, scene)| {
                                    if scene.active { Some(*location) } else { None }
                                },
                            )
                            .collect()
                    }),
                flashed_scenes: self
                    .project
                    .as_mut()
                    .map_or_else(Default::default, |project| {
                        project
                            .scenes_instances_grid
                            .iter_mut()
                            .filter_map(
                                |(location, scene)| {
                                    if scene.flash { Some(*location) } else { None }
                                },
                            )
                            .collect()
                    }),
                available_scenes_grid: self.project.as_mut().map_or_else(
                    Default::default,
                    |project| {
                        project
                            .scenes_instances_grid
                            .iter()
                            .map(|(location, scene_instance)| (*location, scene_instance.color))
                            .collect()
                    },
                ),
                selected_scene_opacity: {
                    let mut beat_progression = self.timing.beat_progression();

                    self.project
                        .as_mut()
                        .and_then(|project| {
                            project
                                .get_scenes_instance(&self.selected_scene_instance)
                                .map(|scene_instance| {
                                    beat_progression += scene_instance
                                        .beat_progression_offset
                                        .value(beat_progression, &self.collections);
                                    scene_instance
                                        .opacity
                                        .value(beat_progression, &self.collections)
                                })
                        })
                        .unwrap_or(1.0)
                },
            }
            .enqueue();
        }

        self.draw_main_window(ctx, None);

        let viewport_ids = self.other_main_windows.clone();
        for viewport_id in viewport_ids {
            ctx.show_viewport_immediate(
                viewport_id,
                default_viewport_builder()
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

        self.windows.update(
            ctx,
            &self.timing,
            self.project.as_mut(),
            &mut self.collections,
        );

        ctx.request_repaint();
    }
}

impl App {
    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn draw_main_window(&mut self, ctx: &egui::Context, viewport_id: Option<ViewportId>) {
        let panel_frame = egui::Frame::new()
            .fill(ctx.style().visuals.window_fill())
            .stroke(ctx.style().visuals.widgets.noninteractive.fg_stroke);

        CentralPanel::default().frame(panel_frame).show(ctx, |ui| {
            if viewport_id.is_none() {
                let callback = Callback::new_paint_callback(Rect::ZERO, RendererCallback);
                ui.painter().add(callback);
            }
            let mut ui = ui.new_child(UiBuilder::new().max_rect(ui.max_rect().shrink(4.0)));

            self.menu(&mut ui, viewport_id);
            self.status_bar(&mut ui, viewport_id);

            if let Some(error) = crate::storage::error() {
                show_storage_error(&mut ui, error);
                return;
            } else if let Some(loading) = crate::storage::loading() {
                show_storage_loading(&mut ui, loading);
                return;
            }

            if self.project.is_some() {
                egui::SidePanel::left("config")
                    .resizable(false)
                    .exact_width(400.0)
                    .show_inside(&mut ui, |ui| self.config(ui));
                egui::TopBottomPanel::top("preview")
                    .resizable(true)
                    .default_height(200.0)
                    .min_height(200.0)
                    .show_inside(&mut ui, |ui| self.preview(ui));
                egui::CentralPanel::default().show_inside(&mut ui, |ui| self.scenes(ui));
            } else {
                self.no_project(&mut ui);
            }
        });
    }
    pub fn new(ui_action_receiver: Receiver<UiAction>) -> Option<Self> {
        let app = Self {
            startup: true,
            timing: Default::default(),
            last_always_render_fps_frame: Instant::now(),
            blackout: true,
            blackout_hold: false,
            selected_scene_instance: Default::default(),
            project: Default::default(),
            project_id: Default::default(),
            windows: Default::default(),
            other_main_windows: Default::default(),
            git_commit_message: Default::default(),
            ui_action_receiver,
            last_title: Default::default(),
            midi_output_active: false,
            palette_asset_tree: AssetTree {
                only_asset_selection: true,
                ..Default::default()
            },
            palette_asset_tree_id: None,
            collections: Default::default(),
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
