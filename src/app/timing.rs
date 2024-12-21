use super::PersistantState;
use egui::{text::LayoutJob, Button, Color32, Rounding, Stroke, TextFormat, Ui, Vec2};
use log::debug;
use std::time::{Duration, Instant};

pub struct Timing {
    pub beats_per_minute: f32,
    last_beat_time: Instant,
    beat_progression: f32,
    avg_fps: Option<f32>,
    avg_fps_time: Instant,
    last_frame: Instant,
    frame_count: usize,
    taps: Vec<Instant>,
    pub fade_mode: FadeMode,
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            beats_per_minute: 60.0,
            last_beat_time: Instant::now(),
            beat_progression: 0.0,
            avg_fps: None,
            avg_fps_time: Instant::now(),
            last_frame: Instant::now(),
            frame_count: 0,
            taps: vec![],
            fade_mode: Default::default(),
        }
    }
}

#[derive(Default, PartialEq, Eq)]
pub enum FadeMode {
    Instant,
    #[default]
    Beat,
    Beats4,
    Beats16,
}

impl Timing {
    pub fn beat_progression(&self) -> f32 {
        self.beat_progression
    }

    pub fn framerate(&self) -> Option<f32> {
        self.avg_fps
    }

    pub fn tick(&mut self) {
        self.limit_fps();
        self.progress_beat();
        self.calculate_avg_fps();
        self.remove_old_taps();
    }

    fn limit_fps(&mut self) {
        let fps_limit = PersistantState::fps_limit();
        let target_frame_time_nanos = 1e+9f32 / fps_limit;
        while target_frame_time_nanos > (self.last_frame.elapsed().as_nanos() as f32) {
            std::thread::sleep(std::time::Duration::from_nanos(100));
        }
        self.last_frame = Instant::now();
    }

    fn progress_beat(&mut self) {
        let now = Instant::now();
        self.beat_progression += (now.duration_since(self.last_beat_time).as_nanos() as f64
            / self.beat_duration_nanoseconds()) as f32;
        self.last_beat_time = now;
    }

    #[inline]
    fn beat_duration_nanoseconds(&self) -> f64 {
        60e+9f64 / self.beats_per_minute as f64
    }

    pub fn fade_duration(&self) -> Duration {
        Duration::from_nanos(
            match self.fade_mode {
                FadeMode::Instant => 0.0,
                FadeMode::Beat => self.beat_duration_nanoseconds(),
                FadeMode::Beats4 => self.beat_duration_nanoseconds() * 4.0,
                FadeMode::Beats16 => self.beat_duration_nanoseconds() * 16.0,
            }
            .round() as u64,
        )
    }

    fn calculate_avg_fps(&mut self) {
        self.frame_count += 1;
        let now = Instant::now();
        if now.duration_since(self.avg_fps_time).as_millis() > 1000 {
            let avg_frame_time = self.avg_fps_time.elapsed() / self.frame_count as u32;
            let fps = 1e+9f32 / (avg_frame_time.as_nanos() as f32);
            self.avg_fps = Some(fps);
            debug!("fps: {fps}");
            self.avg_fps_time = now;
            self.frame_count = 0;
        }
    }

    fn remove_old_taps(&mut self) {
        let Some(last) = self.taps.last() else {
            return;
        };
        let max_age = if self.taps.len() < 2 {
            None
        } else {
            self.taps
                .get(self.taps.len() - 2)
                .map(|tap| last.duration_since(*tap).as_secs_f32())
        }
        .unwrap_or(1.5)
        .min(1.5)
            * 2.0;

        if Instant::now().duration_since(*last).as_secs_f32() > max_age {
            self.taps.clear();
        }
    }

    pub fn half_button(&mut self, ui: &mut Ui) {
        if ui.add(Button::new("x½")).clicked() {
            self.multiply_speed(0.5);
        }
    }

    pub fn double_button(&mut self, ui: &mut Ui) {
        if ui.add(Button::new("x2")).clicked() {
            self.multiply_speed(2.0);
        }
    }

    pub fn multiply_speed(&mut self, multiplier: f32) {
        self.beats_per_minute *= multiplier;
    }

    pub fn add_speed(&mut self, delta: f32) {
        self.beats_per_minute += delta;
    }

    pub fn tap_button(&mut self, ui: &mut Ui, menu_button_size: Vec2, tap_input: bool) {
        let underlined = TextFormat {
            underline: Stroke::new(1.0, Color32::GRAY),
            ..Default::default()
        };
        let mut tap_text = LayoutJob::default();
        tap_text.append("T", 0.0, underlined);
        tap_text.append(
            &format!("ap{}", vec!["."; self.taps.len()].join("")),
            0.0,
            TextFormat::default(),
        );
        let response = ui.add_sized(menu_button_size, Button::new(tap_text));

        let beat_progression = self.beat_progression % 4.0;
        let mut alpha = None;
        if (beat_progression % 1.0) < 0.10 {
            alpha = Some(30.0);
        } else if (beat_progression % 1.0) < 0.20 {
            alpha = Some(20.0 - (((beat_progression % 1.0) - 0.1) * 200.0));
        } else if (beat_progression % 1.0) > 0.9 {
            alpha = Some(((beat_progression % 1.0) - 0.9) * 200.0);
        }
        if let Some(alpha) = alpha {
            ui.painter().rect_filled(
                if (0.5..=1.5).contains(&beat_progression) {
                    // top right
                    response
                        .rect
                        .split_left_right_at_fraction(0.5)
                        .1
                        .split_top_bottom_at_fraction(0.5)
                        .0
                } else if (1.5..=2.5).contains(&beat_progression) {
                    // bottom left
                    response
                        .rect
                        .split_left_right_at_fraction(0.5)
                        .0
                        .split_top_bottom_at_fraction(0.5)
                        .1
                } else if (2.5..=3.5).contains(&beat_progression) {
                    // bottom right
                    response
                        .rect
                        .split_left_right_at_fraction(0.5)
                        .1
                        .split_top_bottom_at_fraction(0.5)
                        .1
                } else {
                    // top left
                    response
                        .rect
                        .split_left_right_at_fraction(0.5)
                        .0
                        .split_top_bottom_at_fraction(0.5)
                        .0
                }
                .shrink(1.0),
                Rounding::default(),
                Color32::from_white_alpha(alpha as u8),
            );
        }

        let tapped = response.clicked() || tap_input;

        if tapped {
            self.tap();
        }
    }

    pub fn tap(&mut self) {
        let now = Instant::now();
        self.taps.push(now);

        if self.taps.len() > 1 {
            if let (Some(first), Some(last)) = (self.taps.first(), self.taps.last()) {
                let beat_time_first = now.duration_since(*first);
                let beat_time_last = now.duration_since(*last);
                let avg_beat_time = (beat_time_first.as_nanos() - beat_time_last.as_nanos())
                    / (self.taps.len() as u128 - 1);

                self.beats_per_minute = (60e+9f64 / f64::from(avg_beat_time as u32)) as f32;

                // adjust beat progression timing to last tap
                let offset = self.beat_progression % 4.0;
                let goal_offset = (self.taps.len() - 1) as f32 % 4.0;
                self.beat_progression += goal_offset - offset;
            }
        }
    }
}
