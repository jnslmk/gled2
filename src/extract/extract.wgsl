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
    let lamp: u32 = global_id.y;

    let position: u32 = positions[universe * LAMPS_PER_UNIVERSE + lamp];
    let position_x: u32 = position & 0xffff0000u;
    let position_y: u32 = position << 16u;
    let position_tex = vec2<f32>(f32(position_x) / 1920., f32(position_y) / 1080.);

    var color: vec3<f32> = textureSampleLevel(tex, sam, position_tex, 0.).rgb;
    if position == 0xffffffffu {
        color = vec3(0.);
    }

    let red = u32(color.r * 255.) & 0x000000ffu;
    let green = u32(color.g * 255.) & 0x000000ffu;
    let blue = u32(color.b * 255.) & 0x000000ffu;

    //rbgr grbg bgrb
    let index = universe * 128u + (3u * (lamp / 4u));
    switch lamp % 4u {
        case 0u, default: {
            output[index] = red | green << 8u | blue << 16u | (output[index] & 0xff000000u);
        }
        case 1u: {
            output[index] = (output[index] & 0x00ffffffu) | red << 24u;
            output[index + 1u] = green | blue << 8u | (output[index + 1u] & 0xffff0000u);
        }
        case 2u: {
            output[index + 1u] = (output[index + 1u] & 0x0000ffffu) | red << 16u | green << 24u;
            output[index + 2u] = blue | (output[index + 2u] & 0xffffff00u);
        }
        case 3u: {
            output[index + 2u] = (output[index + 2u] & 0x000000ffu) | red << 8u | green << 16u | blue << 24u;
        }
    }
}