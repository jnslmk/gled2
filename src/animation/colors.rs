#[derive(Debug, Clone, Default)]
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
            data[i..i + 4].copy_from_slice(&color.red.to_le_bytes());
            data[i + 4..i + 8].copy_from_slice(&color.green.to_le_bytes());
            data[i + 8..i + 12].copy_from_slice(&color.blue.to_le_bytes());
            i += 16;
        }
    }

    pub const fn size() -> usize {
        272
    }
}

#[derive(Debug, Default, Clone)]
pub struct Color {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
}

impl Color {
    pub fn new(red: f32, green: f32, blue: f32) -> Self {
        Self { red, green, blue }
    }
}
