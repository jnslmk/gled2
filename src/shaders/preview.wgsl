@group(0) @binding(0)
// 2 bytes per pixel. Resolution: 1024*1024.
// Each row has 1024*2 = 2048 bytes.
// As it is indexed as u32, each row has 512 entries in the array
var<storage> indices: array<u32, 2097152>; 

@group(0) @binding(1)
var<storage> artnet: array<u32, 4096>;

const TEXTURE_SIZE_U: u32 = 2048u;
const TEXTURE_SIZE_F: f32 = 2048.0;

@fragment
fn fs_main(@location(0) coord: vec2<f32>) -> @location(0) vec4<f32> {
	let x: u32 = u32(round(((1.0 + coord.x) / 2.0) * (TEXTURE_SIZE_F - 1.0)));
	let y: u32 = u32(round(((1.0 - coord.y) / 2.0) * (TEXTURE_SIZE_F - 1.0)));

	let i: u32 = y * (TEXTURE_SIZE_U / 2u) + x / 2u;
	let index = indices[i];
	var universe: u32 = 0u;
	var lamp: u32 = 0u;
	if (x % 2u == 0u) {
		universe = (index & 0xff000000u) >> 24u;
		lamp = (index & 0x00ff0000u) >> 16u;
	} else {
		universe = (index & 0x0000ff00u) >> 8u;
		lamp = index & 0x000000ffu;
	}
	if (universe == 0x000000ffu && lamp == 0x000000ffu) {
		return vec4<f32>(0.);
	} else {
		var red = 0u;
		var green = 0u;
		var blue = 0u;

		let start_byte = lamp * 3u;
		let start_index = universe * 128u + start_byte / 4u;
		switch start_byte % 4u {
			case 0u: {
				red = artnet[start_index] & 0x000000ffu;
				green = (artnet[start_index] & 0x0000ff00u) >> 8u;
				blue = (artnet[start_index] & 0x00ff0000u) >> 16u;
			}
			case 1u: {
				red = (artnet[start_index] & 0x0000ff00u) >> 8u;
				green = (artnet[start_index] & 0x00ff0000u) >> 16u;
				blue = (artnet[start_index] & 0xff000000u) >> 24u;
			}
			case 2u: {
				red = (artnet[start_index] & 0x00ff0000u) >> 16u;
				green = (artnet[start_index] & 0xff000000u) >> 24u;
				blue = artnet[start_index + 1u] & 0x000000ffu;
			}
			case default: {
				red = (artnet[start_index] & 0xff000000u) >> 24u;
				green = artnet[start_index + 1u] & 0x000000ffu;
				blue = (artnet[start_index + 1u] & 0x0000ff00u) >> 8u;
			}
		}

		return vec4<f32>(f32(red) / 255., f32(green) / 255., f32(blue) / 255., 1.);
	}
}