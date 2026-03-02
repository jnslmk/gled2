use crate::{
    audio::sound_data::SoundData,
    storage::{
        collections::Collections,
        curve::multiplied_curve::{MultipliedCurve, RangeDegrees, RangePercentage},
    },
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum FloatValue {
    F32(f32),
    Percentage(MultipliedCurve<RangePercentage>),
    Degrees(MultipliedCurve<RangeDegrees>),
}

impl Eq for FloatValue {}

impl Default for FloatValue {
    fn default() -> Self {
        Self::F32(0.0)
    }
}

impl FloatValue {
    pub fn percentage(&mut self) -> &mut MultipliedCurve<RangePercentage> {
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

    pub fn degrees(&mut self) -> &mut MultipliedCurve<RangeDegrees> {
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

    pub fn value(
        &self,
        beat_progression: f32,
        collection: &Collections,
        sound_data: &SoundData,
    ) -> f32 {
        match self {
            Self::F32(value) => *value,
            Self::Percentage(value) => value.value(beat_progression, collection, sound_data),
            Self::Degrees(value) => value.value(beat_progression, collection, sound_data),
        }
    }
}
