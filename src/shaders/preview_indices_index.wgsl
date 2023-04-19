@group(0) @binding(0)
// Each led position has 2 entries: (x, y).
var<storage> positions: array<f32, 10880>;

@group(0) @binding(1)
// 2 bytes per pixel. Resolution: 1024*1024.
// Each row has 1024*2 = 2048 bytes.
// As it is indexed as u32, each row has 512 entries in the array
var<storage, read_write> indices: array<u32, 2097152>; 

const LAMPS_PER_UNIVERSE: u32 = 170u;
const TEXTURE_SIZE_U: u32 = 2048u;
const TEXTURE_SIZE_F: f32 = 2048.0;
const SQUARE_SIZE: u32 = 2u;

@compute
@workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let universe: u32 = global_id.x;
    let lamp: u32 = global_id.y;
    let start_idx: u32 = universe * LAMPS_PER_UNIVERSE * 2u + lamp * 2u;


    let x: u32 = u32(round(positions[start_idx] * TEXTURE_SIZE_F));
    let y: u32 = u32(round(positions[start_idx + 1u] * TEXTURE_SIZE_F));

    for (var xd = max(0u, x - SQUARE_SIZE); xd < min(TEXTURE_SIZE_U, x + SQUARE_SIZE); xd++) {
        for (var yd = max(0u, y - SQUARE_SIZE); yd < min(TEXTURE_SIZE_U, y + SQUARE_SIZE); yd++) {
            let index = (yd * (TEXTURE_SIZE_U / 2u)) + (xd / 2u);

            if (xd % 2u == 0u) {
                indices[index] = (indices[index] & 0x0000ffffu) | (universe & 0x000000ffu) << 24u | (lamp & 0x000000ffu) << 16u;
            } else {
                indices[index] = (indices[index] & 0xffff0000u) | (universe & 0x000000ffu) << 8u | lamp & 0x000000ffu;
            }
        }
    }
}