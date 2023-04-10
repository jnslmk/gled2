use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ColorPalette {
    /// Max: 16 colors
    pub colors: Vec<Color>,
}

impl ColorPalette {
    /// must be aligned by 16 bytes
    pub fn write_data(&self, data: &mut [u8]) {
        let count = self.colors.len().min(16);

        data[0..4].copy_from_slice(&(count as i32).to_le_bytes());
        // implicit 12 bytes padding
        let mut i = 16;
        for color in self.colors.iter() {
            let rgb = color.rgb();
            data[i..i + 4].copy_from_slice(&rgb[0].to_le_bytes());
            data[i + 4..i + 8].copy_from_slice(&rgb[1].to_le_bytes());
            data[i + 8..i + 12].copy_from_slice(&rgb[2].to_le_bytes());
            i += 16;
        }
    }

    /// must be a multiple of 16
    pub const fn size() -> usize {
        272
    }

    pub fn colors(&mut self) -> &mut Vec<Color> {
        &mut self.colors
    }

    pub fn add_color(&mut self, color: Color) {
        self.colors.push(color);
    }

    pub fn remove_color(&mut self, index: usize) {
        self.colors.remove(index);
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Color {
    rgb: [f32; 3],
}

impl Color {
    pub fn new(red: f32, green: f32, blue: f32) -> Self {
        Self {
            rgb: [red, green, blue],
        }
    }

    pub fn rgb(&self) -> [f32; 3] {
        self.rgb
    }

    pub fn rgb_mut(&mut self) -> &mut [f32; 3] {
        &mut self.rgb
    }
}
