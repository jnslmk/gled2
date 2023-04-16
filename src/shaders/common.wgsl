struct Uniforms {
    // state: 32 bytes
    time: f32,
    beat_progression: f32,
    beats_per_minute: f32,
    frame_rate: f32,
    opacity: f32,
    // padding: 24 bytes

    // colors: 272 bytes
    @align(16) colors_count: i32,
    colors: array<vec3<f32>, 16>,

    // config: 32 bytes
    center_coord: vec2<f32>,
    thickness: f32,
    count: u32,
    direction: u32,
    mode: u32,
    // padding: 16 bytes
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@fragment
fn fs_main(@location(0) coord: vec2<f32>) -> @location(0) vec4<f32> {
    var beat_progression = uniforms.beat_progression;
    if (uniforms.direction != 0u) {
        beat_progression = 1.0 - beat_progression;
    }

    let color = animation(coord, beat_progression);

	return vec4<f32>(color * uniforms.opacity, 1.);
}