@group(0) @binding(0)
// 2 bytes per pixel. Resolution: 2048*2048.
// Each row has 2048*2 = 4096 bytes.
// As it is indexed as u32, each row has 1024 entries in the array
var<storage, read_write> indices: array<u32, 2097152>; 

@compute
@workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    indices[global_id.y * 1024u + global_id.x] = 0xffffffffu;
}