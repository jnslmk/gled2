struct Uniforms {
    // state: 16 bytes
    time: f32,
    beat_progression: f32,
    beats_per_minute: f32,
    frame_rate: f32,

    // colors: 272 bytes
    colors_count: i32,
    colors: array<vec3<f32>, 16>,

    // config: 32 bytes
    center_coord: vec2<f32>,
    thickness: f32,
    count: i32,
    opacity: f32,
    direction: u32
    //..implicit padding: 16 bytes
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;