@group(0) @binding(0)
var<storage> positions: array<u32, 10880>;

@group(0) @binding(1)
var sam: sampler;

@group(0) @binding(2)
var tex: texture_2d<f32>;

@group(0) @binding(3)
var<storage, read_write> output: array<u32, 4096>;

@compute
@workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    //v_indices[global_id.x] = collatz_iterations(v_indices[global_id.x]);
}