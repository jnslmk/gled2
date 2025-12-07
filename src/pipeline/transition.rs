use std::time::{Duration, Instant};

#[derive(Debug, Hash, PartialEq)]
pub struct Transition {
    start: Instant,
    end: Instant,
    goal: TransitionGoal,
}

impl Transition {
    pub fn new(goal: TransitionGoal, duration: Duration) -> Self {
        let now = Instant::now();
        Self {
            start: now,
            end: now + duration,
            goal,
        }
    }

    /// returns None, if the Transition is finished.
    pub fn opacity_factor(&self) -> Option<f32> {
        let now = Instant::now();
        if now > self.end {
            return None;
        }

        let factor = Self::animation(
            (now - self.start).as_nanos() as f64 / (self.end - self.start).as_nanos() as f64,
        ) as f32;

        Some(match self.goal {
            TransitionGoal::TurnOn => factor,
            TransitionGoal::TurnOff => 1.0 - factor,
        })
    }

    pub fn goal(&self) -> TransitionGoal {
        self.goal
    }

    fn animation(x: f64) -> f64 {
        if x < 0.5 {
            2. * x * x
        } else {
            1. - (-2. * x + 2.).powi(2) / 2.
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum TransitionGoal {
    TurnOn,
    TurnOff,
}

impl TransitionGoal {
    pub fn turning_off(&self) -> bool {
        matches!(self, Self::TurnOff)
    }
}
