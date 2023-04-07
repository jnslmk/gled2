fn animation(coord: vec2<f32>, beat_progression: f32) -> vec3<f32> {
    let pos: f32 = 1. - (1.0 + coord.x) / 2.0;
	let dist: f32 = (beat_progression + pos) % 1.;

    return gradient(dist);
}
