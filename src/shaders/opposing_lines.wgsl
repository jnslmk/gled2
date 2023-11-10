fn animation(coord: vec2<f32>, beat_progression: f32) -> vec3<f32> {
    var pos: f32;
    switch(uniforms.mode) {
        case 0u, default: {
            pos = 1. - (1.0 + coord.x) / 2.0;
        }
        case 1u: {
            pos = 1. - (1.0 + coord.y) / 2.0;
        }
    }

    if ((beat_progression + pos) % 1.0 < uniforms.thickness) {
        return uniforms.colors[0];
    } else if (((1.0 - beat_progression) + pos) % 1.0 < uniforms.thickness) {
        return uniforms.colors[1];
    } else {
        return vec3<f32>(0.);
    }
}
