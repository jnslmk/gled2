struct Uniforms {
    // state: 32 bytes
    beat_progression: f32,
    beats_per_minute: f32,
    frame_rate: f32,
    opacity: f32,
    color_shift: f32,
    _not_used_0: f32,
    _not_used_1: f32,
    _not_used_2: f32,

    // colors: 288 bytes
    primary_color: vec3<f32>,
    secondary_color: vec3<f32>,
    gradient_colors: array<vec3<f32>, 16>,

    // config: 32 bytes
    center_coord: vec2<f32>,
    thickness: f32,
    count: u32,
    direction: u32,
    mode: u32,
    speed: f32,
    size: f32,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@fragment
fn fs_main(@location(0) coord: vec2<f32>) -> @location(0) vec4<f32> {
    var beat_progression = (uniforms.beat_progression * uniforms.speed) % 1.0;
    if uniforms.direction == 1u || (uniforms.direction == 2u && (uniforms.beat_progression * uniforms.speed) % 2.0 >= 1.0) {
        beat_progression = 1.0 - beat_progression;
    }

    let color = animation(coord, beat_progression);

    return vec4<f32>(color * uniforms.opacity, 1.);
}

/// Convert rgb color to hsv, apply color shift to hue value and convert it back to rgb
fn apply_color_shift(c: vec3<f32>) -> vec3<f32> {
    var K: vec4<f32> = vec4<f32>(0., -1. / 3., 2. / 3., -1.);
    var p: vec4<f32> = mix(vec4<f32>(c.bg, K.wz), vec4<f32>(c.gb, K.xy), step(c.b, c.g));
    let q: vec4<f32> = mix(vec4<f32>(p.xyw, c.r), vec4<f32>(c.r, p.yzx), step(p.x, c.r));
    let d: f32 = q.x - min(q.w, q.y);
    let e: f32 = 0.0000000001;
    let h: f32 = fract(uniforms.color_shift / 360.0 + abs(q.z + (q.w - q.y) / (6. * d + e)));
    let s: f32 = d / (q.x + e);
    let v: f32 = q.x;
    let K2: vec4<f32> = vec4<f32>(1., 2. / 3., 1. / 3., 3.);
    let p2: vec3<f32> = abs(fract(vec3<f32>(h) + K2.xyz) * 6. - K2.www) - K2.xxx;
    return v * mix(K2.xxx, vec3<f32>(clamp(p2.x, 0., 1.), clamp(p2.y, 0., 1.), clamp(p2.z, 0., 1.)), s);
} 

/// Get color shifted primary color
fn primary_color() -> vec3<f32> {
    return apply_color_shift(uniforms.primary_color);
}

/// Get color shifted secondary color
fn secondary_color() -> vec3<f32> {
    return apply_color_shift(uniforms.secondary_color);
}

/// Get color shifted gradient color by index
fn gradient_color(index: i32) -> vec3<f32> {
    return apply_color_shift(uniforms.gradient_colors[index] % 16);
}