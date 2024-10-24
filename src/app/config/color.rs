use crate::{animation::Color, assets::find_palette};
use egui::{Button, Rect, Ui};
use std::{collections::BTreeSet, path::Path, sync::Arc};

pub fn color_selection(
    ui: &mut Ui,
    palette_path: &Path,
    all_colors: BTreeSet<Color>,
    uses_multiple_colors: bool,
) {
    let mut palette = Arc::unwrap_or_clone(find_palette(palette_path));
    let mut changed = false;

    ui.scope(|ui| {
        ui.horizontal_wrapped(|ui| {
            let rects = palette
                .colors
                .iter_mut()
                .map(|color| {
                    let res = ui.color_edit_button_rgb(color.rgb_mut());
                    if res.changed() {
                        changed = true;
                    }
                    Rect::from_min_max(res.rect.center(), res.rect.right_bottom())
                })
                .collect::<Vec<_>>();
            for (index, rect) in rects.into_iter().enumerate() {
                ui.scope(|ui| {
                    ui.style_mut().spacing.interact_size.y = 10.0;
                    if ui.put(rect, Button::new("-")).clicked() {
                        palette.remove_color(index);
                        changed = true;
                    }
                });
            }
        });

        if palette.colors.is_empty() || (uses_multiple_colors && palette.colors.len() < 16) {
            ui.vertical_centered_justified(|ui| {
                let response = ui.button("Add Color");
                let popup_id = ui.make_persistent_id("color_selection");
                if response.clicked() {
                    ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                }
                let below = egui::AboveOrBelow::Below;
                egui::popup::popup_above_or_below_widget(
                    ui,
                    popup_id,
                    &response,
                    below,
                    egui::PopupCloseBehavior::CloseOnClickOutside,
                    |ui| {
                        ui.set_width_range(200.0..=200.0);

                        ui.horizontal_wrapped(|ui| {
                            for mut color in all_colors {
                                if ui.color_edit_button_rgb(color.rgb_mut()).clicked() {
                                    palette.add_color(color);
                                    changed = true;
                                }
                            }
                        });
                    },
                );
            });
        }
    });

    if changed {
        palette.save(palette_path.to_owned());
    }
}
