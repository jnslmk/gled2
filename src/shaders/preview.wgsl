@group(0) @binding(0)
var<storage> preview_positions: array<f32, 21760>;

@group(0) @binding(1)
var<storage> artnet: array<u32, 4096>;

@fragment
fn fs_main(@location(0) coord: vec2<f32>) -> @location(0) vec4<f32> {


	return vec4<f32>(0.);
}