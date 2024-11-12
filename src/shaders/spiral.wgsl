fn spiral(m: vec2<f32>, size: f32, beat_progression: f32) -> f32 {
	let r: f32 = length(m);
	let a: f32 = atan2(m.y, m.x);
	var v: f32 = sin(size * (sqrt(r) - 2. / size * a - 2. * 3.14159265359  * beat_progression));
    let clamped = clamp(v, 0., 1.);
	
    if (uniforms.mode == 0u) {
        return clamped;
    } else {
        return round(clamped);
    }
}

fn animation(coord: vec2<f32>, beat_progression: f32) -> vec3<f32> {
    let pos: vec2<f32> = (1.0 + coord) / 2.0;
	let v: f32 = spiral(uniforms.center_coord - pos, f32(uniforms.count), beat_progression);

    return vec3(primary_color() * v);
}