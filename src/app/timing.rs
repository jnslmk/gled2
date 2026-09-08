use crate::app::persistent_state::PersistentState;
use egui::{Button, Color32, CornerRadius, Stroke, TextFormat, Ui, Vec2, text::LayoutJob};
use rusty_link::{AblLink, SessionState};
use std::{
    sync::atomic::{AtomicU64, Ordering::Relaxed},
    time::{Duration, Instant},
};
use tracing::debug;

pub static CONNECTED_PEERS: AtomicU64 = AtomicU64::new(0);
pub static LINK_ACTIVE_COLOR: Color32 = Color32::from_rgb(41, 116, 145);
#[cfg(test)]
thread_local! {
    // Per-thread so parallel tests never count each other's sleeps.
    static SLEEP_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Sub-millisecond tail at the end of a frame wait covered by spinning instead
/// of sleeping: OS timers can wake early (notably on macOS), and a spin is the
/// only way to hit the deadline precisely at 120fps (~8.3ms frames).
const SPIN_TAIL: Duration = Duration::from_millis(1);

/// Single indirection over `thread::sleep` so tests can count OS sleeps per frame.
fn frame_sleep(duration: Duration) {
    #[cfg(test)]
    SLEEP_CALLS.with(|calls| calls.set(calls.get() + 1));
    std::thread::sleep(duration);
}

pub struct Timing {
    link: AblLink,
    beats_per_minute: f32,
    previous_change_beats_per_minute: f32,
    pub change_beats_per_minute: f32,
    beat_progression: f32,
    avg_fps: Option<f32>,
    avg_fps_time: Instant,
    last_frame: Instant,
    frame_count: usize,
    total_frames: usize,
    max_frame_nanos: u128,
    taps: [Option<Instant>; 4],
    tap_count: usize,
}

impl std::fmt::Debug for Timing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Timing")
            .field("beats_per_minute", &self.beats_per_minute)
            .field("change_beats_per_minute", &self.change_beats_per_minute)
            .finish_non_exhaustive()
    }
}

impl Default for Timing {
    fn default() -> Self {
        let link = AblLink::new(120.0);
        link.enable(true);

        Self {
            link,
            beats_per_minute: 120.0,
            previous_change_beats_per_minute: 120.0,
            change_beats_per_minute: 120.0,
            beat_progression: 0.0,
            avg_fps: None,
            avg_fps_time: Instant::now(),
            last_frame: Instant::now(),
            frame_count: 0,
            total_frames: 0,
            max_frame_nanos: 0,
            taps: [None; 4],
            tap_count: 0,
        }
    }
}

impl Timing {
    pub fn beat_progression(&self) -> f32 {
        self.beat_progression
    }

    pub fn beats_per_minute(&self) -> f32 {
        self.beats_per_minute
    }

    pub fn framerate(&self) -> Option<f32> {
        self.avg_fps
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn tick(&mut self, fps_limit: f32, persistent_state: &PersistentState) {
        self.limit_fps(fps_limit);
        self.set_link_values(persistent_state);
        self.get_link_values();
        self.calculate_avg_fps();
        self.remove_old_taps();
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn limit_fps(&mut self, fps_limit: f32) {
        // One OS sleep covers the bulk of the frame instead of a 100ns
        // spin-sleep loop; a short spin covers the sub-millisecond tail where
        // timers wake early. An overrun frame skips sleeping entirely.
        if fps_limit > 0.0 {
            let deadline = self.last_frame + Duration::from_secs_f32(1.0 / fps_limit);
            let now = Instant::now();
            if now < deadline {
                let remaining = deadline - now;
                if remaining > SPIN_TAIL {
                    frame_sleep(remaining - SPIN_TAIL);
                }
                while Instant::now() < deadline {
                    std::hint::spin_loop();
                }
            }
        }
        // Full wall-clock time of the previous frame, including the time eframe
        // spent acquiring the surface texture and presenting outside of our own
        // update code. The maximum over a sampling window is a proxy for the
        // worst-case surface wait / stall. The first 10 frames are skipped
        // because startup (window creation, swapchain setup) makes them much
        // slower than steady-state frames.
        self.total_frames += 1;
        if self.total_frames > 10 {
            self.max_frame_nanos = self
                .max_frame_nanos
                .max(self.last_frame.elapsed().as_nanos());
        }
        self.last_frame = Instant::now();
    }

    /// Re-sample the current Ableton Link beat position without running a full
    /// `tick` (no fps limiting, no fps accounting). Used when rendering more than
    /// once per displayed frame so each extra render reflects the freshest beat
    /// position instead of repeating the same instant.
    pub fn refresh_beat(&mut self) {
        self.get_link_values();
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn get_link_values(&mut self) {
        CONNECTED_PEERS.store(self.link.num_peers(), Relaxed);

        let mut session_state = SessionState::default();
        self.link.capture_app_session_state(&mut session_state);
        let now = self.link.clock_micros();
        self.beat_progression = 40.0 + session_state.beat_at_time(now, 4.0) as f32;
        self.beats_per_minute = session_state.tempo() as f32;
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn set_link_values(&mut self, persistent_state: &PersistentState) {
        if persistent_state.ableton_link_read_only() {
            self.previous_change_beats_per_minute = self.beats_per_minute;
            self.change_beats_per_minute = self.beats_per_minute;
            return;
        }

        if self.previous_change_beats_per_minute == self.change_beats_per_minute {
            self.previous_change_beats_per_minute = self.beats_per_minute;
            self.change_beats_per_minute = self.beats_per_minute;
            return;
        }

        let mut session_state = SessionState::default();
        self.link.capture_app_session_state(&mut session_state);
        let now = self.link.clock_micros();
        session_state.set_tempo(self.change_beats_per_minute as f64, now);
        self.link.commit_app_session_state(&session_state);
        self.previous_change_beats_per_minute = self.change_beats_per_minute;
    }

    fn calculate_avg_fps(&mut self) {
        self.frame_count += 1;
        let now = Instant::now();
        if now.duration_since(self.avg_fps_time).as_millis() > 1000 {
            let avg_frame_time = self.avg_fps_time.elapsed() / self.frame_count as u32;
            let fps = 1e+9f32 / (avg_frame_time.as_nanos() as f32);
            self.avg_fps = Some(fps);
            let max_frame_ms = self.max_frame_nanos as f32 / 1e6f32;
            debug!("frame_stats: fps={fps:.1} max_frame_ms={max_frame_ms:.3}");
            self.avg_fps_time = now;
            self.frame_count = 0;
            self.max_frame_nanos = 0;
        }
    }

    fn remove_old_taps(&mut self) {
        let mut reset_tap_count = true;
        for tap in self.taps.iter_mut() {
            if let Some(date) = tap {
                if date.elapsed() > Duration::from_secs(10) {
                    tap.take();
                } else {
                    reset_tap_count = false;
                }
            }
        }

        if reset_tap_count {
            self.tap_count = 0;
        }
    }

    pub fn half_button(
        &mut self,
        ui: &mut Ui,
        tap_input: bool,
        persistent_state: &PersistentState,
    ) {
        let ableton_link_read_only = persistent_state.ableton_link_read_only();
        if ui
            .add_enabled(!ableton_link_read_only, Button::new("x½"))
            .clicked()
            || (tap_input && !ableton_link_read_only)
        {
            self.multiply_speed(0.5, persistent_state);
        }
    }

    pub fn double_button(
        &mut self,
        ui: &mut Ui,
        tap_input: bool,
        persistent_state: &PersistentState,
    ) {
        let ableton_link_read_only = persistent_state.ableton_link_read_only();
        if ui
            .add_enabled(!ableton_link_read_only, Button::new("x2"))
            .clicked()
            || (tap_input && !ableton_link_read_only)
        {
            self.multiply_speed(2.0, persistent_state);
        }
    }

    pub fn multiply_speed(&mut self, multiplier: f32, persistent_state: &PersistentState) {
        if persistent_state.ableton_link_read_only() {
            return;
        }

        self.change_beats_per_minute *= multiplier;
    }

    pub fn add_speed(&mut self, delta: f32, persistent_state: &PersistentState) {
        if persistent_state.ableton_link_read_only() {
            return;
        }

        self.change_beats_per_minute += delta;
    }

    pub fn tap_button(
        &mut self,
        ui: &mut Ui,
        menu_button_size: Vec2,
        tap_input: bool,
        persistent_state: &PersistentState,
    ) {
        let ableton_link_read_only = persistent_state.ableton_link_read_only();
        ui.scope(|ui| {
            ui.style_mut().visuals.widgets.inactive.bg_stroke =
                ui.style().visuals.widgets.noninteractive.bg_stroke;

            let underlined = TextFormat {
                underline: Stroke::new(1.0, Color32::GRAY),
                ..Default::default()
            };
            let mut tap_text = LayoutJob::default();
            tap_text.append("T", 0.0, underlined);
            tap_text.append(
                &format!(
                    "ap{}",
                    match (self.tap_count, self.tap_count % 4) {
                        (0, _) => "",
                        (_, 0) => "/",
                        (_, 1) => "–",
                        (_, 2) => "\\",
                        _ => "|",
                    }
                ),
                0.0,
                TextFormat::default(),
            );
            let response = ui.add_enabled(
                !ableton_link_read_only,
                Button::new(tap_text).min_size(menu_button_size),
            );

            let mut alpha = None;
            let bar_progression = self.beat_progression % 1.0;
            if bar_progression < 0.10 {
                alpha = Some(30.0);
            } else if bar_progression < 0.20 {
                alpha = Some(20.0 - ((bar_progression - 0.1) * 200.0));
            } else if bar_progression > 0.9 {
                alpha = Some((bar_progression - 0.9) * 200.0);
            }
            if let Some(alpha) = alpha {
                ui.painter().rect_filled(
                    match self.beat_flank() {
                        0 =>
                        // top left
                        {
                            response
                                .rect
                                .split_left_right_at_fraction(0.5)
                                .0
                                .split_top_bottom_at_fraction(0.5)
                                .0
                        }
                        1 =>
                        // top right
                        {
                            response
                                .rect
                                .split_left_right_at_fraction(0.5)
                                .1
                                .split_top_bottom_at_fraction(0.5)
                                .0
                        }
                        2 =>
                        // bottom left
                        {
                            response
                                .rect
                                .split_left_right_at_fraction(0.5)
                                .0
                                .split_top_bottom_at_fraction(0.5)
                                .1
                        }
                        3 =>
                        // bottom right
                        {
                            response
                                .rect
                                .split_left_right_at_fraction(0.5)
                                .1
                                .split_top_bottom_at_fraction(0.5)
                                .1
                        }
                        _ => unreachable!(),
                    }
                    .shrink(1.0),
                    CornerRadius::default(),
                    Color32::from_white_alpha(alpha as u8),
                );
            }

            let tapped = response.clicked() || (tap_input && !ableton_link_read_only);

            if tapped {
                self.tap(persistent_state);
            }
        });
    }

    pub fn beat_flank(&self) -> u8 {
        match self.beat_progression % 4.0 {
            0.5..1.5 => 1,
            1.5..2.5 => 2,
            2.5..3.5 => 3,
            _ => 0,
        }
    }

    pub fn tap(&mut self, persistent_state: &PersistentState) {
        if persistent_state.ableton_link_read_only() {
            return;
        }

        let link_now = self.link.clock_micros();

        self.tap_count += 1;
        let now = Instant::now();
        self.taps[0] = Some(now);
        self.taps.sort();

        let first = self
            .taps
            .iter()
            .filter_map(|tap| *tap)
            .next()
            .expect("There must be at least one entry");
        if first == now {
            return;
        }

        let beat_time_first = now.duration_since(first);
        let avg_beat_time = (beat_time_first.as_nanos())
            / (self.taps.iter().filter(|tap| tap.is_some()).count() as u128 - 1);

        let new_beats_per_minute = (60e+9f64 / f64::from(avg_beat_time as u32)) as f32;

        // adjust beat progression timing to last tap
        let offset = self.beat_progression % 4.0;
        let goal_offset = (self.tap_count - 1) as f32 % 4.0;
        let mut session_state = SessionState::default();
        self.link.capture_app_session_state(&mut session_state);
        session_state.set_tempo(new_beats_per_minute as f64, link_now);
        session_state.request_beat_at_time(
            (self.beat_progression + goal_offset - offset) as f64,
            link_now,
            4.0,
        );
        self.link.commit_app_session_state(&session_state);
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn sleep_calls() -> usize {
        SLEEP_CALLS.with(|calls| calls.get())
    }

    fn reset_sleep_calls() {
        SLEEP_CALLS.with(|calls| calls.set(0));
    }

    #[test]
    fn limit_fps_holds_configured_cadence() {
        let mut timing = Timing::default();
        let frames = 10;
        let fps_limit = 200.0;
        let start = Instant::now();
        for _ in 0..frames {
            timing.limit_fps(fps_limit);
        }
        let elapsed = start.elapsed();
        let period = Duration::from_secs_f32(1.0 / fps_limit);
        let target = period * frames;
        assert!(
            elapsed >= target - period,
            "frames ran early: {elapsed:?} for {frames} frames at {fps_limit}fps"
        );
        // 20ms over 10 frames: a systematic over-sleep of 2ms per frame fails,
        // while observed overshoot is ~0 and CI scheduling jitter stays far below.
        assert!(
            elapsed < target + Duration::from_millis(20),
            "frames ran late: {elapsed:?} for {frames} frames at {fps_limit}fps"
        );
    }

    #[test]
    fn limit_fps_sleeps_once_per_frame() {
        let mut timing = Timing::default();
        let frames = 5;
        reset_sleep_calls();
        for _ in 0..frames {
            timing.limit_fps(120.0);
        }
        let calls = sleep_calls();
        // Each frame enters with ~8.3ms minus microsecond-scale test overhead of
        // remaining time, always clearing the 1ms spin-tail threshold: exactly
        // one OS sleep per frame (a >7ms scheduling stall would under-sleep).
        assert_eq!(
            calls, frames,
            "expected exactly one OS sleep per frame: {calls} for {frames} frames"
        );
    }

    #[test]
    fn limit_fps_skips_sleep_on_overrun() {
        let mut timing = Timing {
            last_frame: Instant::now() - Duration::from_secs(1),
            ..Timing::default()
        };
        reset_sleep_calls();
        let start = Instant::now();
        timing.limit_fps(120.0);
        let elapsed = start.elapsed();
        assert_eq!(sleep_calls(), 0, "overrun frame must not sleep");
        assert!(
            elapsed < Duration::from_millis(100),
            "overrun frame blocked: {elapsed:?}"
        );
    }

    #[test]
    fn limit_fps_keeps_frame_accounting() {
        let mut timing = Timing::default();
        for _ in 0..12 {
            timing.limit_fps(1000.0);
        }
        assert_eq!(timing.total_frames, 12);
        assert!(timing.max_frame_nanos > 0);
        assert!(timing.last_frame.elapsed() < Duration::from_millis(100));
    }

    #[test]
    fn avg_fps_reports_after_one_second_window() {
        let mut timing = Timing {
            avg_fps_time: Instant::now() - Duration::from_millis(1100),
            frame_count: 100,
            max_frame_nanos: 5_000_000,
            ..Timing::default()
        };
        timing.calculate_avg_fps();
        let fps = timing
            .avg_fps
            .expect("avg fps must be reported after 1s window");
        assert!((80.0..120.0).contains(&fps), "unexpected fps report: {fps}");
        assert_eq!(timing.frame_count, 0);
        assert_eq!(timing.max_frame_nanos, 0);
    }
}
