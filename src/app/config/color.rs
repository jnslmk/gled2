use crate::storage::{ChangingAsset, Palette};
use egui::Ui;

pub fn palette_editor(ui: &mut Ui, palette: &mut Option<ChangingAsset<Palette>>) {
    let mut changed = false;

    if let Some(palette) = palette {
        ui.scope(|ui| {
            ui.horizontal_wrapped(|ui| {
                palette.data.gradient.iter_mut().for_each(|color| {
                    let res = ui.color_edit_button_rgb(color.rgb_mut());
                    if res.changed() {
                        changed = true;
                    }
                });
            });
        });
    }

    if changed
        && ui
            .button("Save")
            .on_hover_ui(|ui| {
                ui.label("Save the palette to disk");
            })
            .clicked()
    {
        if let Some(palette) = palette.take() {
            palette.save();
        }
    }
}
