fn gradient(dist: f32) -> vec3<f32> {
    let color_index: f32 = dist * 16.;
    let relative_strength: f32 = ((color_index) % (1.));
    let first_color: i32 = i32(floor(color_index)) % 16;
    var second_color: i32 = i32(ceil(color_index)) % 16;

    let color: vec3<f32> = gradient_color(first_color) * (1. - relative_strength) + gradient_color(second_color) * relative_strength;
    return color;
}

fn animation(coord: vec2<f32>, beat_progression: f32) -> vec3<f32> {
    switch(uniforms.mode) {
        case 0u, default: { // linear horizontal
            let pos: f32 = 1. - (1.0 + coord.x) / 2.0;
            let dist: f32 = (beat_progression + pos) % 1.;

            return gradient(dist);
        }
        case 1u: { // linear vertical
            let pos: f32 = 1. - (1. + coord.y) / 2.;
            let dist: f32 = (beat_progression + pos) % 1.;

            return gradient(dist);
        }
		case 2u: { // radial
            let pos: vec2<f32> = (1.0 + coord) / 2.0;
            let dist: f32 = ((1.0 - beat_progression) + length(uniforms.center_coord - pos)) % 1.;

            return gradient(dist);
		}
    }
}
