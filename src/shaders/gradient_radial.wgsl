fn animation(coord: vec2<f32>, beat_progression: f32) -> vec3<f32> {
    let pos: vec2<f32> = (1.0 + coord) / 2.0;
	let dist: f32 = ((1.0 - beat_progression) + length(uniforms.center_coord - pos)) % 1.;
	
    return gradient(dist);
}
