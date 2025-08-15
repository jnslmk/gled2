pub mod float_value;

use float_value::FloatValue;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
#[serde(default)]
pub struct AnimationConfig {
    pub u32_0: u32,
    pub u32_1: u32,
    pub u32_2: u32,
    pub float_0: FloatValue,
    pub float_1: FloatValue,
    pub float_2: FloatValue,
    pub float_3: FloatValue,
    pub float_4: FloatValue,
    pub float_5: FloatValue,
}

impl AnimationConfig {
    pub fn write_data(&self, data: &mut [u8], beat_progression: f32) {
        data[0..4].copy_from_slice(&self.u32_0.to_le_bytes());
        data[4..8].copy_from_slice(&self.u32_1.to_le_bytes());
        data[8..12].copy_from_slice(&self.u32_2.to_le_bytes());
        data[12..16].copy_from_slice(&self.float_0.value(beat_progression).to_le_bytes());
        data[16..20].copy_from_slice(&self.float_1.value(beat_progression).to_le_bytes());
        data[20..24].copy_from_slice(&self.float_2.value(beat_progression).to_le_bytes());
        data[24..28].copy_from_slice(&self.float_3.value(beat_progression).to_le_bytes());
        data[28..32].copy_from_slice(&self.float_4.value(beat_progression).to_le_bytes());
        data[32..36].copy_from_slice(&self.float_5.value(beat_progression).to_le_bytes());
    }

    pub const fn size() -> usize {
        36
    }

    pub fn u32(&mut self, index: usize) -> Option<&mut u32> {
        Some(match index {
            0 => &mut self.u32_0,
            1 => &mut self.u32_1,
            2 => &mut self.u32_2,
            _ => return None,
        })
    }

    pub fn float(&mut self, index: usize) -> Option<&mut FloatValue> {
        Some(match index {
            0 => &mut self.float_0,
            1 => &mut self.float_1,
            2 => &mut self.float_2,
            3 => &mut self.float_3,
            4 => &mut self.float_4,
            5 => &mut self.float_5,
            _ => return None,
        })
    }
}
