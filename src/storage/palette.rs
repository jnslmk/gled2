use super::{
    find_palette,
    tree::{Asset, AssetTrait},
    AssetPath,
};
use egui::Color32;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Palette {
    pub primary: Color,
    pub secondary: Color,
    pub gradient: [Color; 16],
}

impl AssetTrait for Palette {
    fn find(path: &AssetPath) -> Asset<Self> {
        find_palette(path)
    }
}

impl Palette {
    /// must be aligned by 16 bytes
    pub fn write_data(&self, data: &mut [u8]) {
        for (i, color) in std::iter::once(self.primary)
            .chain(std::iter::once(self.secondary))
            .chain(self.gradient.iter().copied())
            .enumerate()
        {
            let rgb = color.rgb();
            data[i * 16..i * 16 + 4].copy_from_slice(&rgb[0].to_le_bytes());
            data[i * 16 + 4..i * 16 + 8].copy_from_slice(&rgb[1].to_le_bytes());
            data[i * 16 + 8..i * 16 + 12].copy_from_slice(&rgb[2].to_le_bytes());
        }
    }

    /// must be a multiple of 16
    pub const fn size() -> usize {
        18 * 16
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Color {
    rgb: [f32; 3],
}

impl PartialOrd for Color {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Color {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.rgb_u32().cmp(&other.rgb_u32())
    }
}
impl PartialEq for Color {
    fn eq(&self, other: &Self) -> bool {
        self.rgb_u32() == other.rgb_u32()
    }
}
impl Eq for Color {}

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

    fn rgb_u32(&self) -> [u32; 3] {
        [
            (self.rgb[0] * 10000.0).round() as u32,
            (self.rgb[1] * 10000.0).round() as u32,
            (self.rgb[2] * 10000.0).round() as u32,
        ]
    }
}

impl From<&Color> for Color32 {
    fn from(color: &Color) -> Self {
        let rgb = color.rgb();
        Color32::from_rgb(
            (rgb[0] * 254.0) as u8,
            (rgb[1] * 254.0) as u8,
            (rgb[2] * 254.0) as u8,
        )
    }
}

impl From<Color> for Color32 {
    fn from(color: Color) -> Self {
        (&color).into()
    }
}
