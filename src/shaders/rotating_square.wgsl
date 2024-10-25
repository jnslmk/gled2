fn animation(coord: vec2<f32>, beat_progression: f32) -> vec3<f32> {
	let a: f32 = 1.5708 * beat_progression;
	let c: f32 = cos(a);
	let s: f32 = sin(a);
	let pos: vec2<f32> = (uniforms.center_coord - (1.0 + coord) / 2.0) / 1.0 * mat2x2<f32>(c, -s, s, c);

	let outer: vec2<f32> = step(abs(pos), vec2(uniforms.size));
    let inner: vec2<f32> = step(abs(pos), vec2(uniforms.size - uniforms.thickness));

	if (inner.x * inner.y == 1.0) {
		return uniforms.secondary_color;
	} else if (outer.x * outer.y == 1.0) {
        return uniforms.primary_color;
    }

	return vec3<f32>(0.0);
} 
