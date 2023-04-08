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
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            beats_per_minute: 130.0,
            fps_limit: 200,
            beat_progression: 0.0,
            fps: None,
            start: Instant::now(),
            last_frame: Instant::now(),
            frame_count: 0,
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
        let duration_milliseconds = 60_000.0 / self.beats_per_minute;
        self.beat_progression = (self.beat_progression
            + now.duration_since(self.last_frame).as_fractional_millis() as f32
                / duration_milliseconds)
            % 1.;
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
}
