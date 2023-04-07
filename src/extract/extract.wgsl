@group(0) @binding(0)
var<storage> positions: array<u32, 10880>;

@group(0) @binding(1)
var sam: sampler;

@group(0) @binding(2)
var tex: texture_2d<f32>;

@group(0) @binding(3)
var<storage, read_write> output: array<u32, 4096>;

const LAMPS_PER_UNIVERSE: u32 = 170u;

@compute
@workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let universe: u32 = global_id.x;

    // we get the colors for 4 lamps at a time as we need to interleafe those to fill up 12 bytes 
    // in this pattern: `rbgr grbg bbgr` as access is only possible with 32bit at a time.
    let idx: u32 = global_id.y;

    var colors: array<u32, 12>;
    for (var i = 0u; i < 4u; i++) {
        let position: u32 = positions[universe * LAMPS_PER_UNIVERSE + idx * 4u + i];
        let position_tex = vec2<f32>(f32(position & 0xffff0000u) / 1920., f32(position << 16u) / 1080.);
        var color: vec3<f32> = textureSampleLevel(tex, sam, position_tex, 0.).rgb;
        if position == 0xffffffffu {
            color = vec3(0.);
        }
        colors[i * 3u] = u32(color.r * 255.) & 0x000000ffu;
        colors[i * 3u + 1u] = u32(color.g * 255.) & 0x000000ffu;
        colors[i * 3u + 2u] = u32(color.b * 255.) & 0x000000ffu;
    }

    let index = universe * 128u + idx * 3u;
    output[index]      = colors[0] | colors[1] << 8u | colors[2] << 16u  | colors[3] << 24u;
    output[index + 1u] = colors[4] | colors[5] << 8u | colors[6] << 16u  | colors[7] << 24u;
    output[index + 2u] = colors[8] | colors[9] << 8u | colors[10] << 16u | colors[11] << 24u;
}