use crate::storage::{RangeDegrees, RangePercentage, StaticOrCurve};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum FloatValue {
    F32(f32),
    Percentage(StaticOrCurve<RangePercentage>),
    Degrees(StaticOrCurve<RangeDegrees>),
}

impl Eq for FloatValue {}

impl Default for FloatValue {
    fn default() -> Self {
        Self::F32(0.0)
    }
}

impl FloatValue {
    pub fn percentage(&mut self) -> &mut StaticOrCurve<RangePercentage> {
        match self {
            Self::Percentage(value) => value,
            _ => {
                *self = Self::Percentage(Default::default());
                if let Self::Percentage(value) = self {
                    value
                } else {
                    unreachable!()
                }
            }
        }
    }

    pub fn degrees(&mut self) -> &mut StaticOrCurve<RangeDegrees> {
        match self {
            Self::Degrees(value) => value,
            _ => {
                *self = Self::Degrees(Default::default());
                if let Self::Degrees(value) = self {
                    value
                } else {
                    unreachable!()
                }
            }
        }
    }

    pub fn value(&self, beat_progression: f32) -> f32 {
        match self {
            Self::F32(value) => *value,
            Self::Percentage(value) => value.value(beat_progression),
            Self::Degrees(value) => value.value(beat_progression),
        }
    }
}
