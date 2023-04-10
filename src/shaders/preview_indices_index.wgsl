@group(0) @binding(0)
// Each led position has 2 entries: (x, y).
var<storage> positions: array<f32, 10880>;

@group(0) @binding(1)
// 2 bytes per pixel. Resolution: 2048*2048.
// Each row has 2048*2 = 4096 bytes.
// As it is indexed as u32, each row has 1024 entries in the array
var<storage, read_write> indices: array<u32, 2097152>; 

const TEXTURE_SIZE: f32 = 2048.0;
const SQUARE_SIZE: u32 = 4u;

@compute
@workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let lamp: u32 = global_id.x;

    let x: u32 = u32(round(positions[lamp * 2u] * TEXTURE_SIZE));
    let y: u32 = u32(round(positions[lamp * 2u + 1u] * TEXTURE_SIZE));

    for (var xd = x - SQUARE_SIZE; xd < x + SQUARE_SIZE; xd++) {
        for (var yd = y - SQUARE_SIZE; yd < y + SQUARE_SIZE; yd++) {
            let index = yd * 1024u + xd / 2u;

            if (xd % 2u == 0u) {
                indices[index] = (indices[index] & 0x0000ffffu) | (lamp << 16u);
            } else {
                indices[index] = (indices[index] & 0xffff0000u) | (lamp & 0x0000ffffu);
            }
        }
    }
}