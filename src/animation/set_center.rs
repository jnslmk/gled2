use egui::{load::SizedTexture, CursorIcon, Image, Sense, TextureId, Ui, Vec2};

pub fn set_center_button(
    ui: &mut Ui,
    center: &mut (f32, f32),
    rendered: TextureId,
    svg: Option<TextureId>,
) {
    ui.vertical_centered_justified(|ui| {
        ui.menu_button("Select center of animation", |ui| {
            let size = 300.0;
            let res = ui.add(
                Image::new(SizedTexture::new(rendered, Vec2::splat(size))).sense(Sense::click()),
            );
            if let Some(svg) = svg {
                ui.put(
                    res.rect,
                    Image::new(SizedTexture::new(svg, Vec2::splat(size))),
                );
            }
            if let Some(pos) = res
                .hover_pos()
                .map(|pos| pos - res.rect.min)
                .filter(|pos| pos.x > 0.0 || pos.y > 0.0 || pos.x < size || pos.y < size)
            {
                ui.output_mut(|o| o.cursor_icon = CursorIcon::Crosshair);
                *center = (pos.x / size, 1.0 - pos.y / size);
            }
            if res.clicked() {
                ui.close_menu();
            }
        });
    });
}
