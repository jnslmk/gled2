fn animation(coord: vec2<f32>, beat_progression: f32) -> vec3<f32> {
    let pos: f32 = 1. - (1. + coord.y) / 2.;
	let dist: f32 = (beat_progression + pos) % 1.;

    return gradient(dist);
}
