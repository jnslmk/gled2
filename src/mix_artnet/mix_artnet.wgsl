@group(0) @binding(0)
var<storage, read_write> artnet: array<u32, 4096>;

@group(0) @binding(1)
var<storage> other: array<u32, 4096>;

@compute
@workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx: u32 = global_id.x;

    let a_0: u32 = artnet[idx] & 0x000000ffu;
    let a_1: u32 = (artnet[idx] >> 8u) & 0x000000ffu;
    let a_2: u32 = (artnet[idx] >> 16u) & 0x000000ffu;
    let a_3: u32 = (artnet[idx] >> 24u) & 0x000000ffu;

    let b_0: u32 = other[idx] & 0x000000ffu;
    let b_1: u32 = (other[idx] >> 8u) & 0x000000ffu;
    let b_2: u32 = (other[idx] >> 16u) & 0x000000ffu;
    let b_3: u32 = (other[idx] >> 24u) & 0x000000ffu;

    let c_0: u32 = (a_0 + b_0) & 0x000000ffu;
    let c_1: u32 = (a_1 + b_1) & 0x000000ffu;
    let c_2: u32 = (a_2 + b_2) & 0x000000ffu;
    let c_3: u32 = (a_3 + b_3) & 0x000000ffu;

    artnet[idx] = c_0 | c_1 << 8u | c_2 << 16u | c_3 << 24u;
}