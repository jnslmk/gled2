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

use crate::input::osc::OSCHandler;
use crate::storage::is_loading;
use crate::{
    audio::{AudioPool, sound_data::SoundData},
    input::{Input, external_control::ExternalControlState},
    output_state::ProjectState,
    pipeline::{extract_output::ExtractOutput, renderer_callback::RendererCallback},
    storage::{
        asset::{Asset, palette::Palette, project::Project, scene::grid::GridLocation},
        asset_id::AssetId,
        collections::Collections,
    },
    ui::{
        action::UiAction, asset_tree::AssetTree, scene_effect_editor::SceneEffectEditorState,
        window_common::default_viewport_builder, windows::Windows,
    },
};
use eframe::egui_wgpu::Callback;
use egui::{CentralPanel, Id, LayerId, Rect, Ui, UiBuilder, ViewportId, ahash::HashSet};
use kanal::{Receiver, Sender};
use persistant_state::PersistantState;
use std::sync::Arc;
use std::time::Instant;
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
    pub selected_scene_effect_editor: SceneEffectEditorState,
    pub git_commit_message: String,
    pub ui_action_receiver: Receiver<UiAction>,
    pub midi_monitor_receiver: kanal::Receiver<crate::midi::monitor::MidiMonitorEvent>,
    pub test_command_sender: Sender<crate::midi::runtime::TestCommand>,
    pub midi_learn_state: crate::midi::learn::LearnState,
    pub last_title: String,
    pub midi_output_active: bool,
    pub palette_asset_tree: AssetTree<Palette>,
    pub palette_asset_tree_id: Option<Id>,
    pub collections: Collections,
    pub network_stats: (f64, f64),
    pub network_stats_receiver: Receiver<(f64, f64)>,
    pub persistant_state: PersistantState,
    pub extract_output: ExtractOutput,
    pub external_control_state: ExternalControlState,
    pub audio_pool: AudioPool,
    pub sound_data: SoundData,
    pub osc_handler: Option<Arc<OSCHandler>>,
}

impl eframe::App for App {
    #[cfg_attr(feature = "profiling", profiling::function)]
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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

        if let Ok(Some(network_stats)) = self.network_stats_receiver.try_recv() {
            self.network_stats = network_stats;
        }
        self.persistant_state.update();
        self.sound_data.update();
        self.timing
            .tick(self.persistant_state.fps_limit(), &self.persistant_state);
        self.collections.update();

        if !is_loading() && self.startup {
            self.startup = false;
            if let Some(project) = self.persistant_state.last_project_id() {
                UiAction::SetProject(project).enqueue();
            }
        }

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

        // update the program state from network signals
        self.external_control_state
            .process_events(&mut self.project, &self.collections);

        if let Some(project) = &mut self.project {
            project.render(
                &self.timing,
                self.blackout || self.blackout_hold,
                if self.persistant_state.effects_always_render() && {
                    self.last_always_render_fps_frame.elapsed().as_secs_f32() > 1.0 / 30.0
                } {
                    self.last_always_render_fps_frame = Instant::now();
                    true
                } else {
                    false
                },
                &self.collections,
                &self.extract_output,
                &self.sound_data,
            );
        }

        ProjectState {
            project: self.project.clone(),
            selected_scene_instance: self.selected_scene_instance,
            blackout: self.blackout || self.blackout_hold,
            beats_per_minute: self.timing.beats_per_minute(),
            beat_progression: self.timing.beat_progression(),
        }
        .enqueue();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        self.draw_main_window(&ctx, None);

        let viewport_ids = self.other_main_windows.clone();
        for viewport_id in viewport_ids {
            ctx.show_viewport_immediate(
                viewport_id,
                default_viewport_builder()
                    .with_inner_size([1300.0, 1024.0])
                    .with_drag_and_drop(true)
                    .with_min_inner_size([300.0, 200.0]),
                |ui, _viewport_class| {
                    ui.ctx().input(|input| {
                        if input.viewport().close_requested() {
                            UiAction::CloseWindow(viewport_id).enqueue();
                        }
                    });

                    self.draw_main_window(ui.ctx(), Some(viewport_id));
                },
            );
        }

        self.windows.update(
            &ctx,
            &self.timing,
            &mut self.project,
            &mut self.collections,
            &mut self.persistant_state,
            &mut self.extract_output,
            &mut self.sound_data,
            &self.midi_monitor_receiver,
            &self.test_command_sender,
            &mut self.midi_learn_state,
        );

        ctx.request_repaint();
    }
}

impl App {
    pub(crate) fn set_selected_scene_instance(&mut self, location: GridLocation) {
        self.selected_scene_instance = location;
        self.selected_scene_effect_editor.reset();
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn draw_main_window(&mut self, ctx: &egui::Context, viewport_id: Option<ViewportId>) {
        let panel_frame = egui::Frame::new()
            .fill(ctx.global_style().visuals.window_fill())
            .stroke(ctx.global_style().visuals.widgets.noninteractive.fg_stroke);

        let mut root_ui = Ui::new(
            ctx.clone(),
            Id::new((ctx.viewport_id(), "main_window_panel")),
            UiBuilder::new()
                .layer_id(LayerId::background())
                .max_rect(ctx.content_rect()),
        );
        root_ui.set_clip_rect(ctx.content_rect());

        CentralPanel::default()
            .frame(panel_frame)
            .show_inside(&mut root_ui, |ui| {
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
                } else if is_loading() {
                    show_storage_loading(&mut ui);
                    return;
                }

                if self.project.is_some() {
                    egui::Panel::left("config")
                        .resizable(false)
                        .exact_size(400.0)
                        .show_inside(&mut ui, |ui| self.config(ui));
                    egui::Panel::top("preview")
                        .resizable(true)
                        .default_size(200.0)
                        .min_size(200.0)
                        .show_inside(&mut ui, |ui| self.preview(ui));
                    egui::CentralPanel::default().show_inside(&mut ui, |ui| self.scenes(ui));
                } else {
                    self.no_project(&mut ui);
                }
            });
    }
    pub fn new(
        ui_action_receiver: Receiver<UiAction>,
        network_stats_receiver: Receiver<(f64, f64)>,
        midi_monitor_receiver: kanal::Receiver<crate::midi::monitor::MidiMonitorEvent>,
        test_command_sender: Sender<crate::midi::runtime::TestCommand>,
        midi_learn_receiver: Receiver<[u8; 3]>,
        extract_output: ExtractOutput,
        artnet_control_receiver: Receiver<Vec<u8>>,
        audio_pool: AudioPool,
    ) -> Option<Self> {
        let app = Self {
            startup: true,
            timing: Default::default(),
            last_always_render_fps_frame: Instant::now(),
            blackout: true,
            blackout_hold: false,
            selected_scene_instance: Default::default(),
            selected_scene_effect_editor: Default::default(),
            project: Default::default(),
            project_id: Default::default(),
            windows: Default::default(),
            other_main_windows: Default::default(),
            git_commit_message: Default::default(),
            ui_action_receiver,
            midi_monitor_receiver,
            test_command_sender,
            midi_learn_state: crate::midi::learn::LearnState::new(midi_learn_receiver),
            last_title: Default::default(),
            midi_output_active: false,
            palette_asset_tree: AssetTree {
                only_asset_selection: true,
                ..Default::default()
            },
            palette_asset_tree_id: None,
            collections: Default::default(),
            network_stats: Default::default(),
            network_stats_receiver,
            persistant_state: Default::default(),
            extract_output,
            external_control_state: ExternalControlState::new(artnet_control_receiver),
            audio_pool,
            sound_data: Default::default(),
            osc_handler: OSCHandler::start().ok(),
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
        let persistant_state = PersistantState::default();

        Self {
            url: persistant_state.git_url(),
            use_passphrase: persistant_state.git_credentials().use_passphrase(),
            passphrase: Default::default(),
        }
    }
}
