use super::{
    all_palettes,
    asset::{Asset, AssetTrait},
    get_palette, set_palette_in_cache, Action, AssetId,
};
use egui::{Color32, Rect, Shape, Vec2};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Palette {
    pub primary: Color,
    pub secondary: Color,
    pub gradient: [Color; 16],
}

impl AssetTrait for Palette {
    const DIR_NAME: &'static str = "palettes";

    fn get(id: &AssetId<Palette>) -> Arc<Asset<Self>> {
        get_palette(id)
    }

    fn all() -> Vec<Arc<Asset<Self>>> {
        all_palettes()
    }

    fn tree_entry_show(&self, ui: &mut egui::Ui) {
        let max = ui.next_widget_position() + Vec2::new(ui.available_width(), 0.0);
        let color_band_rect = Rect::from_min_max(
            ui.next_widget_position().max(max - Vec2::new(200.0, 0.0)),
            max,
        );
        ui.painter()
            .add(Shape::Mesh(self.color_band_mesh(color_band_rect)));
    }

    fn save(asset: Asset<Self>) {
        set_palette_in_cache(asset.clone());

        Action::SavePalette { palette: asset }.send();
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
