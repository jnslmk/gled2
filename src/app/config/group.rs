use crate::app::svg::groups;
use egui::{Button, Color32, RichText, Stroke, Ui};

pub fn selection(ui: &mut Ui, selected_group: &mut String) {
    ui.scope(|ui| {
        ui.style_mut().spacing.interact_size.y = 30.0;
        ui.horizontal_wrapped(|ui| {
            for group in groups() {
                if ui.add(button(&group, selected_group == &group)).clicked() {
                    *selected_group = group;
                };
            }
        })
    });
}

pub fn button(group: &str, selected: bool) -> Button {
    static COLORS: &[Color32] = &[
        Color32::from_rgb(175, 213, 129),
        Color32::from_rgb(177, 152, 221),
        Color32::from_rgb(140, 181, 255),
        Color32::from_rgb(217, 176, 140),
        Color32::from_rgb(217, 148, 140),
        Color32::from_rgb(113, 208, 132),
        Color32::from_rgb(213, 129, 192),
        Color32::from_rgb(163, 218, 224),
        Color32::from_rgb(222, 233, 190),
        Color32::from_rgb(255, 255, 255),
    ];

    let index = groups().iter().position(|g| g == group).unwrap_or_default();
    let button = Button::new(RichText::new(group).color(Color32::from_black_alpha(200)))
        .fill(COLORS[index % COLORS.len()]);
    if selected {
        button.stroke(Stroke::new(3.0, Color32::RED))
    } else {
        button
    }
}
