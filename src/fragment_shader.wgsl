struct Uniforms {
    time: f32,
    colors_count: i32,
    colors: array<vec3<f32>, 16>,
    center_coord: vec2<f32>,
    thickness: f32,
    count: i32,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var<storage, read_write> work_buffer: array<f32, 4096>;

fn gradient(dist: f32) -> vec3<f32> {
    let color_index: f32 = dist * f32(uniforms.colors_count);
	let relative_strength: f32 = ((color_index) % (1.));
	let first_color: i32 = i32(floor(color_index)) % uniforms.colors_count;
	var second_color: i32 = i32(ceil(color_index)) % uniforms.colors_count;

	let color: vec3<f32> = uniforms.colors[first_color] * (1. - relative_strength) + uniforms.colors[second_color] * relative_strength;
	return color;
}

fn radial_gradient(coord: vec2<f32>) -> vec3<f32> {
    let pos: vec2<f32> = (1.0 + coord) / 2.0;
	let dist: f32 = (uniforms.time + length(uniforms.center_coord - pos)) % 1.;
	
    return gradient(dist);
}

fn linear_gradient_horizontal(coord: vec2<f32>) -> vec3<f32> {
    let pos: f32 = 1. - (1.0 + coord.x) / 2.0;
	let dist: f32 = (uniforms.time + pos) % 1.;

    return gradient(dist);
}

fn linear_gradient_vertical(coord: vec2<f32>) -> vec3<f32> {
    let pos: f32 = 1. - (1. + coord.y) / 2.;
	let dist: f32 = (uniforms.time + pos) % 1.;

    return gradient(dist);
}

fn line_sweep(coord: vec2<f32>) -> vec3<f32> {
    let pos: f32 = 1. - (1. + coord.x) / 2.;
    let dist: f32 = (uniforms.time + pos) % 1.;

    if dist < uniforms.thickness {
        return uniforms.colors[0];
    } else {
        return vec3<f32>(0.);
    }
}

fn stripes(coord: vec2<f32>) -> vec3<f32> {
    let empty = 1. / f32(uniforms.count) - uniforms.thickness;
    let pos: f32 = 1. - (1.0 + coord.x) / 2.0;

    if ((uniforms.time + pos) % (empty + uniforms.thickness) < uniforms.thickness) {
        return uniforms.colors[0];
    } else {
        return vec3<f32>(0.);
    }
}

fn stars(coord: vec2<f32>) -> vec3<f32> {
    // * start einmal auf rand
    // * x - jede runde auf rand
    // * y - jede runde auf rand
    // * color - jede runde auf rand
    // * size - jede runde auf rand
    return vec3(0.);
}

fn bla(coord: vec2<f32>) -> vec3<f32> {
    var c: vec3<f32>;
	var l: f32 = 0.;
	var z: f32 = uniforms.time;

	for (var i: i32 = 0; i < 3; i = i + 1) {
		var uv: vec2<f32> = coord;
		var p: vec2<f32> = coord;
		p = p - (0.5);
		p.x = p.x * (16. / 9.);
		z = z + (0.07);
		l = length(p);
		uv = uv + (p / l * (sin(z) + 1.) * abs(sin(l * 9. - z - z)));
		c[i] = 0.01 / length(((uv) % (1.)) - 0.5);
	}

	return vec3<f32>((c / l) * uniforms.time);
}

@fragment
fn fs_main(@location(0) coord: vec2<f32>) -> @location(0) vec4<f32> {
    let color = bla(coord);

	return vec4<f32>(color, 1.);
}
