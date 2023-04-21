#[derive(Debug, Default, Clone, Copy)]
pub struct State {
    /// seconds since the animation started
    pub time: f32,
    /// Progress in current beat: 0.0 -> 1.0
    pub beat_progression: f32,
    /// Beats per minute
    pub beats_per_minute: f32,
    /// Current displayed framerate
    pub framerate: f32,
    /// Opacity of animation: 0.0 -> 1.0
    pub opacity: f32,
}

impl State {
    /// must be aligned by 16 bytes
    pub fn write_data(&self, data: &mut [u8]) {
        data[0..4].copy_from_slice(&self.time.to_le_bytes());
        data[4..8].copy_from_slice(&self.beat_progression.to_le_bytes());
        data[8..12].copy_from_slice(&self.beats_per_minute.to_le_bytes());
        data[12..16].copy_from_slice(&self.framerate.to_le_bytes());
        data[16..20].copy_from_slice(&self.opacity.to_le_bytes());
    }

    /// must be a multiple of 16
    pub const fn size() -> usize {
        16 * 2
    }
}
