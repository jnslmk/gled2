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

use crate::input::external_control::ExternalControlState;
use crate::pipeline::extract_output::ExtractOutput;
use crate::pipeline::output_clear::OutputClear;
use crate::pipeline::preview::Preview;
use crate::pipeline::preview_indices::PreviewIndices;
use crate::pipeline::transition::{Transition, TransitionGoal};
use crate::storage::asset::scene::grid::GridLocation;
use crate::storage::asset::scene::instance::SceneInstance;
use crate::{input::Input, midi::state::MidiState, pipeline::renderer_callback::RendererCallback, storage::{
    asset::{palette::Palette, project::Project, Asset},
    asset_id::AssetId,
    loading,
}, ui::{
    action::UiAction, asset_tree::AssetTree, window_common::default_viewport_builder,
    windows::Windows,
}, wgpu_render_state};
use artnet_protocol::PaddedData;
use eframe::egui_wgpu::Callback;
use egui::{ahash::HashSet, CentralPanel, Id, Rect, UiBuilder, ViewportId};
use persistant_state::PersistantState;
use rand::seq::IndexedMutRandom;
use std::collections::hash_map::ValuesMut;
use std::time::Duration;
use std::{sync::mpsc::Receiver, time::Instant};
use storage::{show_storage_error, show_storage_loading};
use timing::Timing;
use wgpu::{CommandEncoderDescriptor, Queue};

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
    pub external_control_state: ExternalControlState,
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
            match self.project_id.and_then(Asset::get) {
                None => "No project".to_string(),
                Some(project) => project.name().to_string(),
            }
        );
        if title != self.last_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }

        self.external_control_state.process_events();

        self.render();

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
                                        .value(beat_progression);
                                    scene_instance.opacity.value(beat_progression)
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

        self.windows
            .update(ctx, &self.timing, self.project.as_mut());

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
    pub fn new(ui_action_receiver: Receiver<UiAction>, artnet_control_receiver: crossbeam_channel::Receiver<PaddedData>) -> Option<Self> {
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
            external_control_state: ExternalControlState::new(artnet_control_receiver),
        };

        Some(app)
    }


    /// Iterate all scene instances of a projects grid and render them
    fn render(&mut self) {
        if let Some(project) = &mut self.project {
            let timing = &self.timing;
            let blackout = self.blackout || self.blackout_hold;
            let always_render = if PersistantState::effects_always_render() && {
                self.last_always_render_fps_frame.elapsed().as_secs_f32() > 1.0 / 30.0
            } {
                self.last_always_render_fps_frame = Instant::now();
                true
            } else {
                false
            };
            let fade_duration = self.timing.fade_duration();
            let wgpu_render_state = wgpu_render_state();
            let device = wgpu_render_state.device;
            let queue = &wgpu_render_state.queue;

            let external_instances = render_external_scenes(
                &mut self.external_control_state,
                project,
                queue,
                timing);
            let project_instances = render_project(
                project,
                fade_duration,
                queue,
                always_render,
                timing);

            let render_instances = project_instances.chain(external_instances);

            PreviewIndices::get().prepare(queue);

            #[allow(unused_mut)]
            let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Render animations"),
            });

            #[cfg(feature = "profiling")]
            {
                let mut wgpu_profiler = crate::WGPU_PROFILER.lock();
                OutputClear::get().run(&mut wgpu_profiler.scope("OutputClear", &mut encoder));
                for scene_instance in render_instances {
                    scene_instance.render(
                        &mut wgpu_profiler.scope(
                            format!(
                                "Render scene \"{}\"",
                                Asset::get(scene_instance.scene).unwrap_or_default().name()
                            ),
                            &mut encoder,
                        ),
                        blackout,
                        always_render,
                    );
                }
                ExtractOutput::get().run(&mut wgpu_profiler.scope("ExtractOutput", &mut encoder));
                PreviewIndices::get().run(&mut wgpu_profiler.scope("PreviewIndices", &mut encoder));
                Preview::run(&mut wgpu_profiler.scope("Preview", &mut encoder));
                wgpu_profiler.resolve_queries(&mut encoder);
            }

            #[cfg(not(feature = "profiling"))]
            {
                OutputClear::get().run(&mut encoder);
                for scene_instance in render_instances {
                    scene_instance.render(&mut encoder, blackout, always_render);
                }
                ExtractOutput::get().run(&mut encoder);
                PreviewIndices::get().run(&mut encoder);
                Preview::run(&mut encoder);
            }

            RendererCallback::add(encoder.finish());
        }
    }
}

fn render_external_scenes<'a>(
    external_control_state: &'a mut ExternalControlState,
    project: &mut Project,
    queue: &Queue,
    timing: &Timing) -> std::slice::IterMut<'a, SceneInstance> {
    let deck_groups = project.groups.clone();
    let main_dimmer = project.main_dimmer;
    let palette = project.palette.and_then(Asset::get);
    for scene_instance in &mut external_control_state.scene_slots {
        scene_instance.prepare(
            queue,
            false,
            palette.clone(),
            &deck_groups,
            timing,
            main_dimmer,
        );
    }
    external_control_state.scene_slots.iter_mut()
}

fn render_project<'a>(project: &'a mut Project, fade_duration: Duration, queue: &wgpu::Queue, always_render: bool, timing: &Timing)
                      -> ValuesMut<'a, GridLocation, SceneInstance> {
    if project.auto_mode_active {
        if project
            .auto_mode_last_change
            .get_or_insert_with(Instant::now)
            .elapsed()
            .as_secs()
            > project.auto_mode_seconds
        {
            let auto_mode_max_scenes = project.auto_mode_max_scenes;
            let mut prev = std::collections::HashSet::new();
            {
                let mut indices = project
                    .scenes_instances_grid
                    .iter()
                    .filter(|(_index, scene)| scene.active)
                    .map(|(location, _scene)| *location)
                    .collect::<Vec<_>>();

                let mut disable_count =
                    (indices.len() + 1).saturating_sub(auto_mode_max_scenes);
                while disable_count > 0 {
                    if let Some(location) = indices.choose_mut(&mut rand::rng()).copied()
                        && prev.insert(location)
                    {
                        disable_count -= 1;
                        if let Some(scene) = project.scenes_instances_grid.get_mut(&location) {
                            scene.set_transition(Transition::new(
                                TransitionGoal::TurnOff,
                                fade_duration,
                            ));
                        }
                    }
                }
            }

            let mut scenes = project
                .scenes_instances_grid
                .iter_mut()
                .filter(|(location, _scene)| !prev.contains(location))
                .collect::<Vec<_>>();
            if let Some((_index, scene)) = scenes.choose_mut(&mut rand::rng()) {
                scene.set_transition(Transition::new(TransitionGoal::TurnOn, fade_duration));
            }

            project.auto_mode_last_change.take();
        }
    } else {
        project.auto_mode_last_change.take();
    }

    let palette = project.palette.and_then(Asset::get);
    let deck_groups = project.groups.clone();
    let main_dimmer = project.main_dimmer;
    for scene_instance in project.scenes_instances_grid.values_mut() {
        scene_instance.prepare(
            queue,
            always_render,
            palette.clone(),
            &deck_groups,
            timing,
            main_dimmer,
        );
    }
    project.scenes_instances_grid.values_mut()
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
