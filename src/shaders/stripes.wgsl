fn animation(coord: vec2<f32>, beat_progression: f32) -> vec3<f32> {
    let empty = 1. / f32(uniforms.count) - uniforms.thickness;
    let pos: f32 = 1. - (1.0 + coord.x) / 2.0;

    if ((beat_progression + pos) % (empty + uniforms.thickness) < uniforms.thickness) {
        return uniforms.colors[0];
    } else {
        return vec3<f32>(0.);
    }
}
