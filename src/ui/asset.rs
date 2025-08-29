use super::{ChangeButton, asset_tree::AssetTree};
use crate::{
    storage::{
        asset::{Asset, AssetTrait},
        asset_id::AssetId,
    },
    ui::OverwriteChangeButton,
};
use egui::{Ui, UiKind};
use egui_ltreeview::TreeViewState;

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
                    format!("📂 {}", T::NAME)
                },
                |ui| {
                    if let Some(id) =
                        AssetTree::show_asset_selection(ui, ui.make_persistent_id(T::NAME))
                    {
                        *self = Some(id);
                        ui.data_mut(|d| {
                            d.remove::<TreeViewState<usize>>(ui.make_persistent_id(T::NAME))
                        });
                        ui.close_kind(UiKind::Menu);
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

impl<T: AssetTrait> OverwriteChangeButton for Option<AssetId<T>> {
    const NAME: &'static str = T::NAME;
}
