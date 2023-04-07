@group(0) @binding(0)
var<storage, read_write> main: array<u32, 4096>;

@group(0) @binding(1)
var<storage> other: array<u32, 4096>;

@compute
@workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx: u32 = global_id.x;

    main[idx] = main[idx] + other[idx]; // TODO: bytewise mix
}