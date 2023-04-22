fn permute(x: vec4<f32>) -> vec4<f32> {
	return (((34. * x + 1.) * x) % (289.));
} 

fn cellular2x2x2(P: vec3<f32>) -> vec2<f32> {
	let Pi: vec3<f32> = ((floor(P)) % (289.));
	let Pf: vec3<f32> = fract(P);
	let Pfx: vec4<f32> = Pf.x + vec4<f32>(0., -1., 0., -1.);
	let Pfy: vec4<f32> = Pf.y + vec4<f32>(0., 0., -1., -1.);
	var p: vec4<f32> = permute(Pi.x + vec4<f32>(0., 1., 0., 1.));
	p = permute(p + Pi.y + vec4<f32>(0., 0., 1., 1.));
	let p1: vec4<f32> = permute(p + Pi.z);
	let p2: vec4<f32> = permute(p + Pi.z + vec4<f32>(1.));
	let ox1: vec4<f32> = fract(p1 * 0.14285715) - 0.42857143;
	let oy1: vec4<f32> = ((floor(p1 * 0.14285715)) % (7.)) * 0.14285715 - 0.42857143;
	let oz1: vec4<f32> = floor(p1 * 0.020408163) * 0.16666667 - 0.41666666;
	let ox2: vec4<f32> = fract(p2 * 0.14285715) - 0.42857143;
	let oy2: vec4<f32> = ((floor(p2 * 0.14285715)) % (7.)) * 0.14285715 - 0.42857143;
	let oz2: vec4<f32> = floor(p2 * 0.020408163) * 0.16666667 - 0.41666666;
	let dx1: vec4<f32> = Pfx + 0.8 * ox1;
	let dy1: vec4<f32> = Pfy + 0.8 * oy1;
	let dz1: vec4<f32> = Pf.z + 0.8 * oz1;
	let dx2: vec4<f32> = Pfx + 0.8 * ox2;
	let dy2: vec4<f32> = Pfy + 0.8 * oy2;
	let dz2: vec4<f32> = Pf.z - 1. + 0.8 * oz2;
	let d1: vec4<f32> = dx1 * dx1 + dy1 * dy1 + dz1 * dz1;
	var d2: vec4<f32> = dx2 * dx2 + dy2 * dy2 + dz2 * dz2;
	var d: vec4<f32> = min(d1, d2);
	d2 = max(d1, d2);
	var dxy = d.xy;
	if (d.x < d.y) { dxy = d.xy; } else { dxy = d.yx; };
	d.x = dxy.x;
	d.y = dxy.y;
	var dxz = d.xz;
	if (d.x < d.z) { dxz = d.xz; } else { dxz = d.zx; };
	d.x = dxz.x;
	d.z = dxz.y;
	var dxw = d.xw;
	if (d.x < d.w) { dxw = d.xw; } else { dxw = d.wx; };
	d.x = dxw.x;
	d.w = dxw.y;
	var dyzw = d.yzw;
	dyzw = min(d.yzw, d2.yzw);
	d.y = dyzw.x;
	d.z = dyzw.y;
	d.w = dyzw.z;
	d.y = min(d.y, d.z);
	d.y = min(d.y, d.w);
	d.y = min(d.y, d2.x);
	return sqrt(d.xy);
} 

fn animation(coord: vec2<f32>, beat_progression: f32) -> vec3<f32> {
    var pos: vec2<f32> = (1.0 + coord) / 2.0;
	pos = pos * (10.);
	let F: vec2<f32> = cellular2x2x2(vec3<f32>(pos, uniforms.beat_progression));
	let n: f32 = 1.0 - smoothstep(0.3, 0.3 + f32(uniforms.count) * 0.1, F.x);
	return vec3<f32>(uniforms.colors[i32(F.y * f32(uniforms.colors_count)) % uniforms.colors_count] * n);
} 
