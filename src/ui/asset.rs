use super::asset_tree::AssetTree;
use crate::storage::{
    asset::{Asset, AssetTrait},
    asset_id::AssetId,
    collections::Collections,
};
use egui::{Ui, UiKind};
use egui_ltreeview::TreeViewState;

pub trait CollectionsChangeButton {
    fn collections_change_button(&mut self, ui: &mut Ui, collections: &mut Collections) -> bool;
}

impl<T: AssetTrait> CollectionsChangeButton for Option<AssetId<T>> {
    fn collections_change_button(&mut self, ui: &mut Ui, collections: &mut Collections) -> bool {
        let mut changed = false;

        let asset = self.and_then(|id| Asset::get(id, collections));
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
                    if let Some(id) = AssetTree::show_asset_selection(
                        ui,
                        ui.make_persistent_id(T::NAME),
                        collections,
                    ) {
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

impl<T: AssetTrait> CollectionsChangeButton for Option<Option<AssetId<T>>> {
    fn collections_change_button(&mut self, ui: &mut Ui, collections: &mut Collections) -> bool {
        let mut changed = false;

        let mut overwrite = self.is_some();
        ui.checkbox(&mut overwrite, format!("Overwrite {}", T::NAME));
        if self.is_none() && overwrite {
            *self = Some(Some(Default::default()));
            changed = true;
        } else if self.is_some() && !overwrite {
            *self = None;
            changed = true;
        }
        if let Some(asset_id) = self {
            ui.vertical_centered_justified(|ui| {
                changed |= asset_id.collections_change_button(ui, collections);
            });
        }

        changed
    }
}
