fn animation(coord: vec2<f32>, beat_progression: f32) -> vec3<f32> {
    let empty = 1. / f32(uniforms.count) - uniforms.thickness;

    var pos: f32;
    switch(uniforms.mode) {
        case 0u, default: {
            pos = 1. - (1.0 + coord.x) / 2.0;
        }
        case 1u: {
            pos = 1. - (1.0 + coord.y) / 2.0;
        }
    }

    if ((beat_progression + pos) % (empty + uniforms.thickness) < uniforms.thickness) {
        return uniforms.colors[i32((beat_progression + pos) / (empty + uniforms.thickness)) % uniforms.colors_count];
    } else {
        return vec3<f32>(0.);
    }
}
