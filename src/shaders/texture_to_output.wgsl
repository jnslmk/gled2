@group(0) @binding(0)
var<storage> positions: array<f32, 10880>;

@group(0) @binding(1)
var sam: sampler;

@group(0) @binding(2)
var tex: texture_2d<f32>;

@group(0) @binding(3)
var<storage, read_write> output: array<u32, 4096>;

struct Uniforms {
    // colors: 288 bytes
    primary_color: vec3<f32>,
    secondary_color: vec3<f32>,
    gradient_colors: array<vec3<f32>, 16>,

    // state: 64 bytes
    random: f32,
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
};

@group(0) @binding(4)
var<uniform> uniforms: Uniforms;

const LAMPS_PER_UNIVERSE: u32 = 170u;

@compute
@workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let universe: u32 = global_id.x;

    // we get the colors for 4 lamps at a time as we need to interleafe those to fill up 12 bytes 
    // in this pattern: `rbgr grbg bbgr` as access is only possible with 32bit at a time.
    let idx: u32 = global_id.y;
    let start_idx: u32 = universe * LAMPS_PER_UNIVERSE * 2u + idx * 8u;

    var colors: array<u32, 12>;
    for (var i = 0u; i < 4u; i++) {
        if (idx == 42u && i > 1u) {
            continue;
        }

        let x: f32 = positions[start_idx + i * 2u];
        let y: f32 = positions[start_idx + i * 2u + 1u];

        let position_tex = vec2<f32>(x, y);
        var color: vec3<f32> = textureSampleLevel(tex, sam, position_tex, 0.).rgb;
        if 2. - x < 0.001 && 2. - y < 0.001 {
            color = vec3(0.);
        }
        colors[i * 3u] = u32(round(color.r * uniforms.opacity * 254.)) & 0x000000ffu;
        colors[i * 3u + 1u] = u32(round(color.g * uniforms.opacity * 254.)) & 0x000000ffu;
        colors[i * 3u + 2u] = u32(round(color.b * uniforms.opacity * 254.)) & 0x000000ffu;
    }

    let index = universe * 128u + idx * 3u;
    output[index]      = colors[0] | colors[1] << 8u | colors[2] << 16u  | colors[3] << 24u;
    if (idx < 42u) {
        output[index + 1u] = colors[4] | colors[5] << 8u | colors[6] << 16u  | colors[7] << 24u;
        output[index + 2u] = colors[8] | colors[9] << 8u | colors[10] << 16u | colors[11] << 24u;
    } else {
        output[index + 1u] = colors[4] | colors[5] << 8u;
    }
}