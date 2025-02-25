use egui::{Align2, Color32, CornerRadius, Pos2, Rect, TextStyle, Ui, Vec2, WidgetText};

pub fn show_pills(ui: &mut Ui, start: Pos2, texts: Vec<(String, Color32)>) {
    const PADDING: f32 = 5.0;
    let mut pills = vec![];
    for (text, bg_color) in texts {
        let size = WidgetText::from(egui::RichText::new(text.clone()))
            .into_galley(ui, None, ui.available_width(), TextStyle::Body)
            .rect
            .size()
            + Vec2::new(4.0, 0.0);
        pills.push((text, bg_color, size));
    }

    let total_width = pills
        .iter()
        .map(|(_text, _bg_color, size)| size.x)
        .sum::<f32>()
        + (pills.len().saturating_sub(1)) as f32 * PADDING;

    let mut start = start + Vec2::new(-total_width - 2.0, 0.0);
    for (text, bg_color, size) in pills {
        let rect = Rect::from_min_size(start, size);
        ui.painter()
            .rect_filled(rect, CornerRadius::same(5), bg_color);
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            text,
            egui::TextStyle::Body.resolve(ui.style()),
            Color32::from_black_alpha(200),
        );
        start += Vec2::new(size.x + PADDING, 0.0);
    }
}
