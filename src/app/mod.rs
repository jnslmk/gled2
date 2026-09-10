pub mod config;
pub mod effects;
pub mod menu;
pub mod no_project;
pub mod persistent_state;
pub mod preview;
pub mod project_functions;
pub mod status_bar;
pub mod storage;
pub mod svg;
pub mod timing;

use crate::input::osc::{OSCHandler, OscConfig};
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
use egui::{CentralPanel, Id, LayerId, Rect, Ui, UiBuilder, ViewportId};
use kanal::{Receiver, Sender};
use persistent_state::PersistentState;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};
use storage::{show_storage_error, show_storage_loading};
use timing::Timing;

/// Ignore construction-to-first-frame work without delaying detection of the
/// compositor's first 1 Hz callback.
const PAUSE_GRACE: Duration = Duration::from_millis(500);
/// Smallest logic-call gap treated as "the event loop is externally paced".
/// Comfortably above any legitimate display cadence, yet far below the ~1 s
/// drip a Wayland compositor imposes on windows it does not show.
const STARVE_MIN_GAP: Duration = Duration::from_millis(150);
/// Largest starvation threshold. This leaves 100 ms below the measured 1 Hz
/// compositor drip while allowing more than two configured periods at 3 fps.
const STARVE_MAX_GAP: Duration = Duration::from_millis(900);
/// Consecutive starved logic calls before painting is paused: two in a row
/// mean a steady external throttle (the ~1 s drip repeats every frame), not a
/// one-off stall.
const STARVED_FRAMES_TO_PAUSE: u8 = 2;

/// The logic-call gap that counts as starved, scaled by the fps limiter and
/// capped below the compositor's measured ~1 s drip.
///
/// A configured period at or above the cap leaves no safe margin below the
/// drip, so the watchdog is disabled rather than falsely pausing a visible UI.
fn starve_threshold(fps_limit: f32) -> Option<Duration> {
    let limiter_period_secs = if fps_limit.is_finite() && fps_limit > 0.0 {
        1.0 / fps_limit
    } else {
        0.0
    };
    if limiter_period_secs >= STARVE_MAX_GAP.as_secs_f32() {
        return None;
    }

    let limiter_period = Duration::from_secs_f32(limiter_period_secs);
    Some((limiter_period * 4).clamp(STARVE_MIN_GAP, STARVE_MAX_GAP))
}

fn has_real_interaction(input: &egui::InputState) -> bool {
    input.events.iter().any(|event| {
        matches!(
            event,
            egui::Event::WindowFocused(true)
                | egui::Event::PointerMoved(..)
                | egui::Event::PointerButton { .. }
                | egui::Event::MouseWheel { .. }
                | egui::Event::Key { .. }
                | egui::Event::Touch { .. }
        )
    })
}

/// Keep native child windows declared while bypassing their immediate renderers.
///
/// eframe still runs the root UI pass when a child is visible, even while
/// `SKIP_PAINTING` suppresses surface presentation. An immediate viewport would
/// paint inline during that pass, so temporarily declare existing children as
/// deferred, input-only viewports instead. Repainting them consumes pending
/// child input without presenting their surfaces.
fn declare_paused_viewports(ctx: &egui::Context) {
    let viewport_ids = ctx.input(|input| {
        input
            .raw
            .viewports
            .keys()
            .copied()
            .filter(|viewport_id| *viewport_id != ViewportId::ROOT)
            .collect::<Vec<_>>()
    });

    for viewport_id in viewport_ids {
        let builder = ctx.viewport_for(viewport_id, |viewport| viewport.builder.clone());
        ctx.show_viewport_deferred(viewport_id, builder, |ui, _viewport_class| {
            let close_requested = ui.ctx().input(|input| input.viewport().close_requested());
            if close_requested {
                // eframe clears this input event after the deferred pass. Sending
                // Close queues it again for the restored immediate callback.
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if close_requested || ui.ctx().input(has_real_interaction) {
                eframe::SKIP_PAINTING.store(false, std::sync::atomic::Ordering::Relaxed);
                ui.ctx().request_repaint_of(ViewportId::ROOT);
            }
        });
        ctx.request_repaint_of(viewport_id);
    }
}

pub struct App {
    pub startup: bool,
    pub windows: Windows,
    pub timing: Timing,
    pub last_always_render_fps_frame: Instant,
    /// Wall-clock anchor of the previous `App::logic` call. A gap far beyond
    /// the frame cadence means the event loop is being paced externally - a
    /// Wayland compositor drips ~1 frame/s to a window it does not show -
    /// which would pin animations and output to that rate. See the watchdog
    /// in `App::logic`.
    last_logic_frame: Instant,
    /// Consecutive `App::logic` calls whose gap exceeded the starve
    /// threshold; painting pauses once this reaches `STARVED_FRAMES_TO_PAUSE`.
    starved_logic_frames: u8,
    /// When the app was created; the watchdog ignores first-frame setup.
    started_at: Instant,
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
    pub persistent_state: PersistentState,
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
        self.persistent_state.update();
        self.sound_data.update();
        self.timing
            .tick(self.persistent_state.fps_limit(), &self.persistent_state);
        self.collections.update();

        if !is_loading() && self.startup {
            self.startup = false;
            if let Some(project) = self.persistent_state.last_project_id() {
                UiAction::SetProject(project).enqueue();
            }
        }

        // Keep the show running when the window is not on screen.
        //
        // eframe runs `App::logic` once per frame callback, and a Wayland
        // compositor paces a window it does not show at ~1 frame/s instead of
        // the refresh rate (measured: exactly once per second on niri). That
        // would pin animations, Art-Net and DMX output to ~1 fps as soon as
        // the console is hidden or covered. The compositor never reports an
        // error while dripped - every 1 Hz paint succeeds - so the
        // `on_surface_status` handler in main.rs never fires and 8d93838's
        // pause never engaged. Detect the drip from the actual logic cadence
        // instead, and pause painting. While painting is paused eframe stops
        // requesting compositor frame callbacks, so `App::logic` free-runs at
        // the fps limiter's rate and the show - timing, scene renders, DMX
        // readback and send - continues unchanged.
        let now = Instant::now();
        let frame_gap = now.saturating_duration_since(self.last_logic_frame);
        self.last_logic_frame = now;
        if !eframe::skip_painting()
            && self.started_at.elapsed() > PAUSE_GRACE
            && starve_threshold(self.persistent_state.fps_limit())
                .is_some_and(|threshold| frame_gap > threshold)
        {
            self.starved_logic_frames = self.starved_logic_frames.saturating_add(1);
            if self.starved_logic_frames >= STARVED_FRAMES_TO_PAUSE {
                tracing::debug!(
                    "Logic cadence starved ({frame_gap:?} per frame); window not visible - \
                     pausing painting, output continues"
                );
                eframe::SKIP_PAINTING.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        } else {
            self.starved_logic_frames = 0;
        }

        // Resume painting as soon as the window is interactive again - any
        // input event, or regaining focus, means it is back on screen. Until
        // then `logic` keeps running, so animations and output carry on at the
        // fps limiter's rate with no window on screen.
        // Only real interaction means the window is back on screen. A steady
        // `focused` flag or stray window events are not enough: while painting
        // is paused there is no frame-callback pacing to observe, so a wrong
        // guess cannot be corrected by cadence and would freeze a visible
        // window until the drip resumes.
        if eframe::skip_painting() && ctx.input(has_real_interaction) {
            eframe::SKIP_PAINTING.store(false, std::sync::atomic::Ordering::Relaxed);
        }
        // Painting may be paused without a full UI pass, so keep the event
        // loop ticking from `logic`.
        ctx.request_repaint();

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

        self.render_project();
        // Optionally render (and therefore output) a second time on the same
        // displayed frame. The beat position is re-sampled in between so the
        // extra render reflects the freshest timing, doubling the Art-Net/DMX
        // output rate without waiting for another surface present.
        if self.persistent_state.double_render() {
            self.timing.refresh_beat();
            self.render_project();
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

        // `SKIP_PAINTING` is sampled by eframe before `logic`, so it can become
        // true during this same pass. Keep child declarations alive, but avoid
        // their immediate renderers: unlike eframe's root renderer, immediate
        // viewport renderers do not consult `SKIP_PAINTING`.
        if eframe::skip_painting() {
            declare_paused_viewports(&ctx);
            return;
        }

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
            &mut self.persistent_state,
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
    fn render_project(&mut self) {
        if let Some(project) = &mut self.project {
            project.render(
                &self.timing,
                self.blackout || self.blackout_hold,
                if self.persistent_state.effects_always_render() && {
                    self.last_always_render_fps_frame.elapsed().as_secs_f32() > 1.0 / 30.0
                } {
                    self.last_always_render_fps_frame = Instant::now();
                    true
                } else {
                    false
                },
                &self.collections,
                &mut self.extract_output,
                &self.sound_data,
            );
        }
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
            .show(&mut root_ui, |ui| {
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
                        .show(&mut ui, |ui| self.config(ui));
                    egui::Panel::top("preview")
                        .resizable(true)
                        .default_size(200.0)
                        .min_size(200.0)
                        .show(&mut ui, |ui| self.preview(ui));
                    egui::CentralPanel::default().show(&mut ui, |ui| self.scenes(ui));
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
            last_logic_frame: Instant::now(),
            starved_logic_frames: 0,
            started_at: Instant::now(),
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
            persistent_state: Default::default(),
            extract_output,
            external_control_state: ExternalControlState::new(artnet_control_receiver),
            audio_pool,
            sound_data: Default::default(),
            osc_handler: None,
        };

        Some(app)
    }

    pub fn apply_osc_config(&mut self, config: OscConfig) {
        if let Some(handler) = self.osc_handler.take() {
            handler.stop();
        }
        if config.active {
            self.osc_handler = OSCHandler::start(config.port).ok();
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
        let persistent_state = PersistentState::default();

        Self {
            url: persistent_state.git_url(),
            use_passphrase: persistent_state.git_credentials().use_passphrase(),
            passphrase: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starve_threshold_sits_between_healthy_and_dripped_cadence() {
        assert!(
            PAUSE_GRACE < Duration::from_secs(1),
            "startup grace must expire before the compositor's first 1 Hz callback"
        );
        assert!(
            STARVE_MAX_GAP < Duration::from_secs(1),
            "threshold cap must stay below the compositor's 1 Hz drip"
        );

        for fps_limit in [2.0, 3.0, 10.0, 30.0, 60.0, 120.0, 5000.0] {
            let limiter_period = Duration::from_secs_f32(1.0 / fps_limit);
            let threshold =
                starve_threshold(fps_limit).expect("cadence faster than the drip must be watched");
            assert!(
                limiter_period < threshold,
                "threshold {threshold:?} at {fps_limit} fps would flag healthy limiter cadence"
            );
            assert!(
                (STARVE_MIN_GAP..=STARVE_MAX_GAP).contains(&threshold),
                "threshold {threshold:?} at {fps_limit} fps escaped its bounds"
            );
        }

        assert_eq!(starve_threshold(3.0), Some(STARVE_MAX_GAP));
        assert_eq!(starve_threshold(0.0), Some(STARVE_MIN_GAP));
        assert_eq!(
            starve_threshold(1.0),
            None,
            "a configured cadence no faster than the drip must not pause a visible window"
        );
    }

    #[test]
    fn paused_viewports_preserve_close_and_resume_on_child_input() {
        let ctx = egui::Context::default();
        ctx.set_embed_viewports(false);
        let child = ViewportId(Id::new("paused child viewport"));

        let initial_output = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.ctx().show_viewport_deferred(
                child,
                egui::ViewportBuilder::default(),
                |_ui, _viewport_class| {},
            );
        });
        assert!(initial_output.viewport_output.contains_key(&child));
        initial_output.drop_without_applying_deltas();

        let mut root_input = egui::RawInput::default();
        root_input.viewports.insert(child, Default::default());
        let paused_output = ctx.run_ui(root_input, |ui| declare_paused_viewports(ui.ctx()));
        let child_callback = paused_output.viewport_output[&child]
            .viewport_ui_cb
            .clone()
            .expect("paused child must use a deferred input callback");
        paused_output.drop_without_applying_deltas();

        eframe::SKIP_PAINTING.store(true, std::sync::atomic::Ordering::Relaxed);
        let mut child_input = egui::RawInput {
            viewport_id: child,
            ..Default::default()
        };
        child_input.viewports.insert(child, Default::default());
        child_input
            .events
            .push(egui::Event::PointerMoved(egui::Pos2::ZERO));
        ctx.run_ui(child_input, |ui| child_callback(ui))
            .drop_without_applying_deltas();
        let resumed = !eframe::skip_painting();
        assert!(resumed, "real child input must resume painting");

        eframe::SKIP_PAINTING.store(true, std::sync::atomic::Ordering::Relaxed);
        let mut close_info = egui::ViewportInfo::default();
        close_info.events.push(egui::ViewportEvent::Close);
        let mut close_input = egui::RawInput {
            viewport_id: child,
            ..Default::default()
        };
        close_input.viewports.insert(child, close_info);
        let close_output = ctx.run_ui(close_input, |ui| child_callback(ui));
        let close_forwarded = close_output.viewport_output[&child]
            .commands
            .contains(&egui::ViewportCommand::Close);
        let close_resumed = !eframe::skip_painting();
        close_output.drop_without_applying_deltas();

        eframe::SKIP_PAINTING.store(false, std::sync::atomic::Ordering::Relaxed);
        assert!(
            close_forwarded,
            "paused child close must be replayed to its immediate callback"
        );
        assert!(close_resumed, "paused child close must resume the root UI");
    }
}
