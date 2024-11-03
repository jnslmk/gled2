use super::{asset_tree::AssetTree, ChangeButton};
use crate::storage::{Asset, AssetId, AssetTrait};
use egui::Ui;

impl<T: AssetTrait> ChangeButton for Option<AssetId<T>> {
    fn change_button(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;

        let asset = self.map(Asset::get);
        let response = ui
            .vertical_centered_justified(|ui| {
                ui.menu_button(
                    if asset.is_some() {
                        "".to_owned()
                    } else {
                        format!("📂 Select {}", T::NAME)
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
        if let Some(asset) = asset {
            asset.data.show(ui, response.rect);
        }

        changed
    }
}
