use egui::{text::LayoutJob, Button, Color32, Key, Modifiers, Stroke, TextFormat, Ui, Vec2};
use floating_duration::TimeAsFloat;
use log::debug;
use std::time::{Duration, Instant};

pub struct Timing {
    pub beats_per_minute: f32,
    pub fps_limit: u64,
    beat_progression: f32,
    fps: Option<f32>,
    start: Instant,
    last_frame: Instant,
    frame_count: usize,
    tap: Instant,
    freeze: bool,
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            beats_per_minute: 60.0,
            fps_limit: 120,
            beat_progression: 0.0,
            fps: None,
            start: Instant::now(),
            last_frame: Instant::now(),
            frame_count: 0,
            tap: Instant::now(),
            freeze: false,
        }
    }
}

impl Timing {
    pub fn beat_progression(&self) -> f32 {
        self.beat_progression
    }

    pub fn framerate(&self) -> Option<f32> {
        self.fps
    }

    pub fn calculate(&mut self) {
        let now = Instant::now();
        let wait_until = self.last_frame + Duration::from_nanos(1_000_000_000 / self.fps_limit);
        if now < wait_until {
            let wait = wait_until - now;
            log::debug!("Waiting for {:?} ms", wait);
            std::thread::sleep(wait);
        }

        if !self.freeze {
            let duration_milliseconds = 60_000.0 / self.beats_per_minute;
            self.beat_progression = (self.beat_progression
                + now.duration_since(self.last_frame).as_fractional_millis() as f32
                    / duration_milliseconds)
                % 1.;
        }

        self.last_frame = now;
        self.frame_count += 1;
        let dur = now.duration_since(self.start).as_fractional_secs() as f32;
        if dur >= 1.0 {
            let fps = self.frame_count as f32 / dur;
            self.fps = Some(fps);
            debug!("fps: {fps}");
            self.start = now;
            self.frame_count = 0;
        }
    }

    pub fn beat_button(&mut self, ctx: &egui::Context, ui: &mut Ui, menu_button_size: Vec2) {
        let beat_progression = self.beat_progression();
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
        tap_text.append("ap", 0.0, TextFormat::default());
        let mut tap = Button::new(tap_text);
        if let Some(alpha) = alpha {
            tap = tap.fill(Color32::from_white_alpha(alpha as u8));
        }
        if ui.add_sized(menu_button_size, tap).clicked()
            || !ctx.wants_keyboard_input()
                && ctx.input_mut(|i| {
                    i.consume_key(Modifiers::NONE, Key::Space)
                        || i.consume_key(Modifiers::NONE, Key::T)
                })
        {
            let now = Instant::now();
            self.beats_per_minute =
                60000.0 / now.duration_since(self.tap).as_fractional_millis() as f32;
            self.tap = now;
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
