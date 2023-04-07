fn gradient(dist: f32) -> vec3<f32> {
    let color_index: f32 = dist * f32(uniforms.colors_count);
	let relative_strength: f32 = ((color_index) % (1.));
	let first_color: i32 = i32(floor(color_index)) % uniforms.colors_count;
	var second_color: i32 = i32(ceil(color_index)) % uniforms.colors_count;

	let color: vec3<f32> = uniforms.colors[first_color] * (1. - relative_strength) + uniforms.colors[second_color] * relative_strength;
	return color;
}

fn animation(coord: vec2<f32>, beat_progression: f32) -> vec3<f32> {
    let pos: vec2<f32> = (1.0 + coord) / 2.0;
	let dist: f32 = ((1.0 - beat_progression) + length(uniforms.center_coord - pos)) % 1.;
	
    return gradient(dist);
}
