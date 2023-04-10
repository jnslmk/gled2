@group(0) @binding(0)
// 2 bytes per pixel. Resolution: 1024*1024.
// Each row has 1024*2 = 2048 bytes.
// As it is indexed as u32, each row has 512 entries in the array
var<storage, read_write> indices: array<u32, 524288>; 

const TEXTURE_SIZE_U: u32 = 1024u;

@compute
@workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    indices[global_id.y * (TEXTURE_SIZE_U / 2u) + global_id.x] = 0xffffffffu;
}