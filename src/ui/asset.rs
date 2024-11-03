use super::{asset_tree::AssetTree, ChangeButton};
use crate::storage::{Asset, AssetId, AssetTrait};
use egui::Ui;

impl<T: AssetTrait> ChangeButton for Option<AssetId<T>> {
    fn change_button(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;

        let asset = self.and_then(Asset::get);
        let response = ui
            .menu_button(
                if let Some(asset) = asset.as_ref() {
                    if T::SHOW_NAME_IF_SELECTED {
                        asset.name().to_owned()
                    } else {
                        "".to_owned()
                    }
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
            .response;
        if let Some(asset) = asset {
            asset.data.show(ui, response.rect);
        }

        changed
    }
}
