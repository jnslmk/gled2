fn roundLookingBlob(pos: vec2<f32>, tPos: vec2<f32>, r: f32) -> f32 {
	return pow(max(1. - length(pos - tPos), 0.), r);
}

const PI: f32 = 3.14159265359;

fn animation(coord: vec2<f32>, beat_progression: f32) -> vec3<f32> {
    let pos: vec2<f32> = uniforms.center_coord - (1.0 + coord) / 2.0;
    let time = uniforms.beat_progression * PI * 2.;

	var v: f32 = roundLookingBlob(pos, vec2<f32>(sin(time) * 0.4, cos(time) * 0.4), 7.);
	v = v + (roundLookingBlob(pos, vec2<f32>(sin(time * 0.6) * 0.2, cos(time) * 0.3), 6.));
	v = v + (roundLookingBlob(pos, vec2<f32>(cos(time * 0.8) * 0.7, sin(time * 1.1) * 0.4), 5.));
	v = v + (roundLookingBlob(pos, vec2<f32>(cos(time * 0.2) * 0.2, sin(time * 0.9) * 0.5), 8.));
	v = clamp((v - 0.5) * 1000., 0., 1.);
	return vec3<f32>(uniforms.colors[0] * v);
} 
