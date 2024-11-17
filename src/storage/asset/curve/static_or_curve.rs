use std::marker::PhantomData;

use super::Curve;
use crate::{
    storage::{Asset, AssetId, AssetTrait},
    ui::{asset_tree::AssetTree, ChangeButton},
};
use egui_ltreeview::TreeViewState;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub enum StaticOrCurve<R: Range> {
    Static(f32, #[serde(skip)] PhantomData<R>),
    Curve(AssetId<Curve>, #[serde(skip)] PhantomData<R>),
}

impl<R: Range> StaticOrCurve<R> {
    pub const fn new_static(value: f32) -> Self {
        Self::Static(value, PhantomData)
    }

    pub const fn new_curve(bytes: [u8; 16]) -> Self {
        Self::Curve(AssetId::from_uuid(Uuid::from_bytes(bytes)), PhantomData)
    }
}

pub trait Range: Eq {
    const MAX: f32;
    fn format(num: f32) -> String;
}

impl<R: Range> StaticOrCurve<R> {
    pub fn value(&self, beat_progression: f32) -> f32 {
        match self {
            Self::Static(value, ..) => *value,
            Self::Curve(curve, ..) => Asset::get(*curve)
                .map(|curve| curve.data.value(beat_progression % 4.0) * R::MAX)
                .unwrap_or_default(),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct RangePercentage;
impl Range for RangePercentage {
    const MAX: f32 = 1.0;
    fn format(num: f32) -> String {
        format!("{:.1}%", num * 100.0)
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct RangeDegrees;
impl Range for RangeDegrees {
    const MAX: f32 = 360.0;
    fn format(num: f32) -> String {
        format!("{num:.0}°")
    }
}

impl<R: Range> Default for StaticOrCurve<R> {
    fn default() -> Self {
        Self::Static(R::MAX, PhantomData)
    }
}

impl<R: Range> Eq for StaticOrCurve<R> {}

impl<R: Range> ChangeButton for StaticOrCurve<R> {
    fn change_button(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        let rect = ui
            .menu_button(
                match self {
                    Self::Static(value, ..) => format!("Static: {}", R::format(*value)),
                    Self::Curve(curve, ..) => match Asset::get(*curve) {
                        Some(curve) => format!("Curve: {}", curve.path.join("/")),
                        None => "Curve: Not found".to_string(),
                    },
                },
                |ui| {
                    ui.horizontal(|ui| {
                        let mut use_static = matches!(self, Self::Static(..));
                        if ui
                            .radio_value(&mut use_static, true, "Static")
                            .on_hover_text("Use a static value")
                            .changed()
                        {
                            *self = Self::Static(R::MAX, PhantomData);
                            changed = true;
                        }

                        let mut use_curve = matches!(self, Self::Curve(..));
                        if ui
                            .radio_value(&mut use_curve, true, "Curve")
                            .on_hover_text("Use a curve to animate the value")
                            .changed()
                        {
                            *self = Self::Curve(AssetId::new(), PhantomData);
                            changed = true;
                        }
                    });

                    match self {
                        Self::Static(value, ..) => {
                            ui.horizontal(|ui| {
                                changed |= ui
                                    .add(
                                        egui::Slider::new(value, 0.0..=R::MAX)
                                            .custom_formatter(|n, _| R::format(n as f32)),
                                    )
                                    .changed();
                            });
                        }
                        Self::Curve(curve, ..) => {
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
            )
            .response
            .rect;
        if let Self::Curve(curve, ..) = self {
            if let Some(curve) = Asset::get(*curve) {
                curve.data.show(ui, rect);
            }
        }

        changed
    }
}
