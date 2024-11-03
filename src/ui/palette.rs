use super::{asset_tree::AssetTree, ChangeButton};
use crate::storage::{Asset, AssetId, Palette};
use egui::{
    epaint::{Vertex, WHITE_UV},
    Color32, Mesh, Rect, Shape, Ui, Vec2,
};

impl ChangeButton for Option<AssetId<Palette>> {
    fn change_button(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;

        let palette = self.map(Asset::get);
        let response = ui
            .vertical_centered_justified(|ui| {
                ui.menu_button(
                    if palette.is_some() {
                        ""
                    } else {
                        "📂 Select Palette"
                    },
                    |ui| {
                        if let Some(id) = AssetTree::show_asset_selection(ui) {
                            *self = Some(id);
                            ui.close_menu();
                            changed = true;
                        }
                    },
                )
                .response
            })
            .inner;
        if let Some(palette) = palette {
            ui.painter()
                .add(Shape::mesh(palette.data.color_band_mesh(response.rect)));
        }

        changed
    }
}

impl Palette {
    pub fn color_band_mesh(&self, rect: Rect) -> Mesh {
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
    }
}
