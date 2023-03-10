struct Uniforms {
    time: f32,
    colors_count: i32,
    colors: array<vec3<f32>, 16>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var<storage, read_write> work_buffer: array<f32, 4096>;

fn radial_gradient(coord: vec2<f32>) -> vec3<f32> {
    let pos: vec2<f32> = (1.0 + coord) / 2.0;
	let dist: f32 = ((uniforms.time + length(vec2<f32>(0.5) - pos)) % (1.));
	let color_index: f32 = dist * f32(uniforms.colors_count);
	let relative_strength: f32 = ((color_index) % (1.));
	let first_color: i32 = i32(floor(color_index)) % uniforms.colors_count;
	var second_color: i32 = i32(ceil(color_index)) % uniforms.colors_count;

	let color: vec3<f32> = uniforms.colors[first_color] * (1. - relative_strength) + uniforms.colors[second_color] * relative_strength;
	return color;
}

@fragment
fn fs_main(@location(0) coord: vec2<f32>) -> @location(0) vec4<f32> {
    let color = radial_gradient(coord);
    
	return vec4<f32>(color, 1.);
}
