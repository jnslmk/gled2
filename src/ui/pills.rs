use egui::{Color32, Label, Pos2, Rect, Rounding, TextStyle, Ui, Vec2, WidgetText};

pub fn show_pills(ui: &mut Ui, start: Pos2, texts: Vec<(String, Color32)>) {
    const PADDING: f32 = 5.0;
    let mut pills = vec![];
    for (text, bg_color) in texts {
        let text = egui::RichText::new(text).color(Color32::from_black_alpha(200));
        let size = WidgetText::from(text.clone())
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
            .rect_filled(rect, Rounding::same(5.0), bg_color);
        ui.put(rect, Label::new(text).selectable(false));
        start += Vec2::new(size.x + PADDING / 2.0, 0.0);
    }
}
