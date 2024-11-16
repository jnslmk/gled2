use super::Curve;
use crate::{
    storage::{Asset, AssetId, AssetTrait},
    ui::{asset_tree::AssetTree, ChangeButton},
};
use egui_ltreeview::TreeViewState;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub enum StaticOrCurve {
    Static(f32),
    Curve(AssetId<Curve>),
}

impl StaticOrCurve {
    pub fn value(&self, beat_progression: f32) -> f32 {
        match self {
            Self::Static(value) => *value,
            Self::Curve(curve) => Asset::get(*curve)
                .map(|curve| curve.data.value(beat_progression % 4.0))
                .unwrap_or_default(),
        }
    }
}

impl Default for StaticOrCurve {
    fn default() -> Self {
        Self::Static(1.0)
    }
}

impl Eq for StaticOrCurve {}

impl ChangeButton for StaticOrCurve {
    fn change_button(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        ui.menu_button(
            match self {
                Self::Static(value) => format!("Static: {value:.2}"),
                Self::Curve(curve) => match Asset::get(*curve) {
                    Some(curve) => format!("Curve: {}", curve.name()),
                    None => "Curve: Not found".to_string(),
                },
            },
            |ui| {
                ui.horizontal(|ui| {
                    let mut use_static = matches!(self, Self::Static(_));
                    if ui
                        .radio_value(&mut use_static, true, "Static")
                        .on_hover_text("Use a static value")
                        .changed()
                    {
                        *self = Self::Static(1.0);
                        changed = true;
                    }

                    let mut use_curve = matches!(self, Self::Curve(_));
                    if ui
                        .radio_value(&mut use_curve, true, "Curve")
                        .on_hover_text("Use a curve to animate the value")
                        .changed()
                    {
                        *self = Self::Curve(AssetId::new());
                        changed = true;
                    }
                });

                match self {
                    Self::Static(value) => {
                        ui.horizontal(|ui| {
                            changed |= ui
                                .add(egui::Slider::new(value, 0.0..=1.0).text("Value"))
                                .changed();
                        });
                    }
                    Self::Curve(curve) => {
                        ui.horizontal(|ui| {
                            if let Some(id) = AssetTree::show_asset_selection(
                                ui,
                                ui.make_persistent_id(Curve::NAME),
                            ) {
                                *curve = id;
                                ui.data_mut(|d| {
                                    d.remove::<TreeViewState<usize>>(
                                        ui.make_persistent_id(Curve::NAME),
                                    )
                                });
                                ui.close_menu();
                                changed = true;
                            }
                        });
                    }
                }
            },
        );

        changed
    }
}
