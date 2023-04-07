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
    var beat_progression = uniforms.beat_progression;
    if (uniforms.direction == 1u) {
        beat_progression = 1.0 - beat_progression;
    }
	let dist: f32 = (beat_progression + length(uniforms.center_coord - pos)) % 1.;
	
    return gradient(dist);
}

fn linear_gradient_horizontal(coord: vec2<f32>) -> vec3<f32> {
    let pos: f32 = 1. - (1.0 + coord.x) / 2.0;
    var beat_progression = uniforms.beat_progression;
    if (uniforms.direction == 1u) {
        beat_progression = 1.0 - beat_progression;
    }
	let dist: f32 = (beat_progression + pos) % 1.;

    return gradient(dist);
}

fn linear_gradient_vertical(coord: vec2<f32>) -> vec3<f32> {
    let pos: f32 = 1. - (1. + coord.y) / 2.;
    var beat_progression = uniforms.beat_progression;
    if (uniforms.direction == 1u) {
        beat_progression = 1.0 - beat_progression;
    }
	let dist: f32 = (beat_progression + pos) % 1.;

    return gradient(dist);
}

fn line_sweep(coord: vec2<f32>) -> vec3<f32> {
    let pos: f32 = 1. - (1. + coord.x) / 2.;
    var beat_progression = uniforms.beat_progression;
    if (uniforms.direction == 1u) {
        beat_progression = 1.0 - beat_progression;
    }
    let dist: f32 = (beat_progression + pos) % 1.;

    if dist < uniforms.thickness {
        return uniforms.colors[0];
    } else {
        return vec3<f32>(0.);
    }
}

fn stripes(coord: vec2<f32>) -> vec3<f32> {
    let empty = 1. / f32(uniforms.count) - uniforms.thickness;
    let pos: f32 = 1. - (1.0 + coord.x) / 2.0;

    var beat_progression = uniforms.beat_progression;
    if (uniforms.direction == 1u) {
        beat_progression = 1.0 - beat_progression;
    }
    if ((beat_progression + pos) % (empty + uniforms.thickness) < uniforms.thickness) {
        return uniforms.colors[0];
    } else {
        return vec3<f32>(0.);
    }
}

@fragment
fn fs_main(@location(0) coord: vec2<f32>) -> @location(0) vec4<f32> {
    let color = line_sweep(coord);

	return vec4<f32>(color * uniforms.opacity, 1.);
}
