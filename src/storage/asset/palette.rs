use super::AssetTrait;
use egui::{
    Color32, Mesh, Rect, Shape, Vec2,
    epaint::{Vertex, WHITE_UV},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use zerocopy::{FromBytes, Immutable};

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Palette {
    pub primary: Color,
    pub secondary: Color,
    pub gradient: [Color; 16],
}

impl AssetTrait for Palette {
    const DIR_NAME: &'static str = "palettes";
    const NAME: &'static str = "Palette";
    const SHOW_NAME_IF_SELECTED: bool = false;

    fn show(&self, ui: &mut egui::Ui, rect: Rect) {
        let mesh = {
            const COLOR_GRADIENT_RATIO: f32 = 0.35;
            const GAP: f32 = 2.0;
            const BORDER: f32 = 1.0;
            const HEIGHT: f32 = 18.0;
            let available_width = rect.width() - GAP - 4.0 * BORDER;
            let color_width = available_width * COLOR_GRADIENT_RATIO;
            let gradient_with: f32 = available_width - color_width;
            let width_per_gradient_color = gradient_with / 16.0;
            let start_pos = rect.right_center();

            let mut mesh = Mesh::default();
            mesh.add_colored_rect(
                Rect::from_min_max(
                    start_pos
                        - Vec2::new(
                            BORDER + color_width + BORDER + GAP + BORDER + gradient_with + BORDER,
                            HEIGHT / 2.0,
                        ),
                    start_pos - Vec2::new(GAP + BORDER + gradient_with + BORDER, -HEIGHT / 2.0),
                ),
                Color32::BLACK,
            );
            let index = mesh.vertices.len() as u32;
            mesh.add_triangle(index, index + 1, index + 2);
            mesh.add_triangle(index + 3, index + 4, index + 5);
            let primary_color = self.primary.into();
            // Top left
            mesh.vertices.push(Vertex {
                pos: start_pos
                    - Vec2::new(
                        color_width + BORDER + GAP + BORDER + gradient_with + BORDER,
                        HEIGHT / 2.0 - BORDER,
                    ),
                uv: WHITE_UV,
                color: primary_color,
            });
            // Top right
            mesh.vertices.push(Vertex {
                pos: start_pos
                    - Vec2::new(
                        BORDER + GAP + BORDER + gradient_with + BORDER,
                        HEIGHT / 2.0 - BORDER,
                    ),
                uv: WHITE_UV,
                color: primary_color,
            });
            // Bottom left
            mesh.vertices.push(Vertex {
                pos: start_pos
                    - Vec2::new(
                        color_width + BORDER + GAP + BORDER + gradient_with + BORDER,
                        -HEIGHT / 2.0 + BORDER,
                    ),
                uv: WHITE_UV,
                color: primary_color,
            });
            let secondary_color = self.secondary.into();
            // Top right
            mesh.vertices.push(Vertex {
                pos: start_pos
                    - Vec2::new(
                        BORDER + GAP + BORDER + gradient_with + BORDER,
                        HEIGHT / 2.0 - BORDER,
                    ),
                uv: WHITE_UV,
                color: secondary_color,
            });
            // Bottom left
            mesh.vertices.push(Vertex {
                pos: start_pos
                    - Vec2::new(
                        color_width + BORDER + GAP + BORDER + gradient_with + BORDER,
                        -HEIGHT / 2.0 + BORDER,
                    ),
                uv: WHITE_UV,
                color: secondary_color,
            });
            // Bottom right
            mesh.vertices.push(Vertex {
                pos: start_pos
                    - Vec2::new(
                        BORDER + GAP + BORDER + gradient_with + BORDER,
                        -HEIGHT / 2.0 + BORDER,
                    ),
                uv: WHITE_UV,
                color: secondary_color,
            });
            // Gradient background
            mesh.add_colored_rect(
                Rect::from_min_max(
                    start_pos - Vec2::new(BORDER + gradient_with + BORDER, HEIGHT / 2.0),
                    start_pos - Vec2::new(BORDER, -HEIGHT / 2.0),
                ),
                Color32::BLACK,
            );
            // Gradient colors
            for (i, color) in self.gradient.into_iter().rev().enumerate() {
                mesh.add_colored_rect(
                    Rect::from_min_max(
                        start_pos
                            - Vec2::new(
                                width_per_gradient_color * (i + 1) as f32 + BORDER,
                                HEIGHT / 2.0 - BORDER,
                            ),
                        start_pos
                            - Vec2::new(
                                width_per_gradient_color * i as f32 + BORDER,
                                -HEIGHT / 2.0 + BORDER,
                            ),
                    ),
                    color.into(),
                );
            }

            mesh
        };
        ui.painter().add(Shape::Mesh(Arc::new(mesh)));
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

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, FromBytes, Immutable)]
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
