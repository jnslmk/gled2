struct Uniforms {
    // state: 16 bytes
    beat_progression: f32,
    beats_per_minute: f32,
    frame_rate: f32,
    opacity: f32,

    // colors: 272 bytes
    colors_count: i32,
    colors: array<vec3<f32>, 16>,

    // config: 32 bytes
    center_coord: vec2<f32>,
    thickness: f32,
    count: u32,
    direction: u32,
    mode: u32,
    speed: f32,
    // padding: 8 bytes
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@fragment
fn fs_main(@location(0) coord: vec2<f32>) -> @location(0) vec4<f32> {
    var beat_progression = (uniforms.beat_progression * uniforms.speed) % 1.0;
    if (uniforms.direction == 1u || (uniforms.direction == 2u && (uniforms.beat_progression * uniforms.speed) % 2.0 > 1.0)) {
        beat_progression = 1.0 - beat_progression;
    }

    let color = animation(coord, beat_progression);

	return vec4<f32>(color * uniforms.opacity, 1.);
}