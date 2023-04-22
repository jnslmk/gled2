use egui::{text::LayoutJob, Button, Color32, Key, Modifiers, Stroke, TextFormat, Ui, Vec2};
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
    freeze: bool,
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
            freeze: false,
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

    pub fn tick(&mut self, fps_limit: f32) {
        self.limit_fps(fps_limit);
        self.progress_beat();
        self.calculate_avg_fps();
        self.remove_old_taps();
    }

    fn limit_fps(&mut self, fps_limit: f32) {
        let target_frame_time_nanos = 1e+9f32 / fps_limit;
        while target_frame_time_nanos > (self.last_frame.elapsed().as_nanos() as f32) {
            std::thread::sleep(std::time::Duration::from_nanos(100));
        }
        self.last_frame = Instant::now();
    }

    fn progress_beat(&mut self) {
        if !self.freeze {
            let now = Instant::now();
            self.beat_progression += (now.duration_since(self.last_beat_time).as_nanos() as f64
                / self.beat_duration_nanoseconds()) as f32;
            self.last_beat_time = now;
        }
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
        if self
            .taps
            .last()
            .map(|tap| Instant::now().duration_since(*tap).as_millis() > 1000)
            .unwrap_or_default()
        {
            self.taps.clear();
        }
    }

    pub fn beat_button(&mut self, ctx: &egui::Context, ui: &mut Ui, menu_button_size: Vec2) {
        let beat_progression = self.beat_progression() % 1.0;
        let mut alpha = None;
        if beat_progression < 0.10 {
            alpha = Some(30.0);
        } else if beat_progression < 0.20 {
            alpha = Some(20.0 - ((beat_progression - 0.1) * 200.0));
        } else if beat_progression > 0.9 {
            alpha = Some((beat_progression - 0.9) * 200.0);
        }

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
        let mut tap = Button::new(tap_text);
        if let Some(alpha) = alpha {
            tap = tap.fill(Color32::from_white_alpha(alpha as u8));
        }

        let tapped = ui.add_sized(menu_button_size, tap).clicked()
            || !ctx.wants_keyboard_input()
                && ctx.input_mut(|i| {
                    i.consume_key(Modifiers::NONE, Key::Space)
                        || i.consume_key(Modifiers::NONE, Key::T)
                });

        if tapped {
            let now = Instant::now();
            self.taps.push(now);

            if self.taps.len() > 1 {
                if let (Some(first), Some(last)) = (self.taps.first(), self.taps.last()) {
                    let beat_time_first = now.duration_since(*first);
                    let beat_time_last = now.duration_since(*last);
                    let avg_beat_time = (beat_time_first.as_nanos() - beat_time_last.as_nanos())
                        / (self.taps.len() as u128 - 1);

                    self.beats_per_minute = (60e+9f64 / f64::from(avg_beat_time as u32)) as f32;
                }
            }
        }
    }

    pub fn freeze_button(&mut self, ctx: &egui::Context, ui: &mut Ui, menu_button_size: Vec2) {
        let underlined = TextFormat {
            underline: Stroke::new(1.0, Color32::GRAY),
            ..Default::default()
        };
        let mut freeze_text = LayoutJob::default();
        freeze_text.append("F", 0.0, underlined);
        freeze_text.append("reeze", 0.0, TextFormat::default());

        let mut freeze = Button::new(freeze_text);
        if self.freeze {
            freeze = freeze.fill(Color32::DARK_RED);
        }
        if ui.add_sized(menu_button_size, freeze).clicked()
            || !ctx.wants_keyboard_input()
                && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F))
        {
            self.freeze = !self.freeze;
        }
    }
}
