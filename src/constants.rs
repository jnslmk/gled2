/// One Artnet Universe can hold up to 512 Positions. As we only support RGB (for now), we can have up to 170 lamps in a universe (=510 Positions).
pub const TEXTURE_SIZE: u16 = 1024;
pub const PREVIEW_TEXTURE_SIZE: u16 = 2048;
pub const UNIVERSES: u64 = 32;
pub const LAMPS_PER_UNIVERSE: u64 = 170;
pub const LAMPS: u64 = UNIVERSES * LAMPS_PER_UNIVERSE;
pub const POSITIONS_BUFFER_SIZE: u64 = LAMPS * 8;
pub const UNIVERSE_BUFFER_SIZE: u64 = 512;
pub const ARTNET_BUFFER_SIZE: u64 = UNIVERSES * UNIVERSE_BUFFER_SIZE;
pub const PREVIEW_INDICES_BUFFER_SIZE: u64 =
    PREVIEW_TEXTURE_SIZE as u64 * PREVIEW_TEXTURE_SIZE as u64 * 2;
pub const GPU_NOT_INIT: &str = "init_gpu was not yet run :/";
