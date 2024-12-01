struct Uniforms {
    // colors: 288 bytes
    primary_color: vec3<f32>,
    secondary_color: vec3<f32>,
    gradient_colors: array<vec3<f32>, 16>,

    // state: 64 bytes
    beat_progression: f32,
    beats_per_minute: f32,
    frame_rate: f32,
    opacity: f32,
    color_shift: f32,
    speed: f32,
    u32_0: u32,
    u32_1: u32,
    u32_2: u32,
    f32_0: f32,
    f32_1: f32,
    f32_2: f32,
    f32_3: f32,
    f32_4: f32,
    f32_5: f32,
    f32_6: f32,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@fragment
fn fs_main(@location(0) coord: vec2<f32>) -> @location(0) vec4<f32> {
    var beat_progression = (uniforms.beat_progression * uniforms.speed) % 1.0;

    let color = animation(coord, beat_progression);

    if color.r == 0.0 && color.g == 0.0 && color.b == 0.0 {
        return vec4<f32>(0.0);
    }

    return vec4<f32>(color, 1.);
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