struct Uniforms {
    // Audio data: 1024 bytes (256 frequency bins, stored as 64 vec4s for alignment)
    audio_data: array<vec4<f32>, 64>,
}
;

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

const TEXTURE_SIZE_U: u32 = 2048u;
const TEXTURE_SIZE_F: f32 = 2048.0;
const Y_LINES: i32 = 9;
const X_LINES: i32 = 9;

@fragment
fn fs_main(@location(0) coord: vec2<f32>) -> @location(0) vec4<f32> {
    let bin = log2(audio_bin(i32(coord.x * 256)) + 1);
    if (bin > coord.y) {
        return vec4<f32>(1.0, 0.0, 1.0, 1.0);
    } else {
        let y_line_pos = round(coord.y * f32(Y_LINES)) / f32(Y_LINES);
        let x_line_pos = round(coord.x * f32(X_LINES)) / f32(X_LINES);
        if abs(coord.y - y_line_pos) < 0.002 || abs(coord.x - x_line_pos) < 0.002 {
            return vec4<f32>(0.3, 0.3, 0.3, 1.0);
        }

        return vec4<f32>(0.0);
    }
}

/// Get audio FFT frequency bin value (0.0 to 1.0)
/// index: frequency bin index (0-255)
fn audio_bin(index: i32) -> f32 {
    let vec4_index = index / 4;
    let component = index % 4;
    let vec = uniforms.audio_data[vec4_index % 64];
    if component == 0 {
        return vec.x;
    } else if component == 1 {
        return vec.y;
    } else if component == 2 {
        return vec.z;
    } else {
        return vec.w;
    }
}
