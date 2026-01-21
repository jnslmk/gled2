use super::Curve;
use crate::audio::ReactiveSignalHandle;
use crate::{
    storage::{Asset, AssetId, AssetTrait},
    ui::{asset_tree::AssetTree, gled_slider::GledSlider},
};
use egui::{
    containers::menu::{MenuButton, MenuConfig}, Button, Color32,
    UiKind,
};
use egui_ltreeview::TreeViewState;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(default)]
pub struct MultipliedCurve<R: Range> {
    #[serde(alias = "Static", deserialize_with = "deserialize_static")]
    multiplier: f32,
    #[serde(alias = "Curve", deserialize_with = "deserialize_curve")]
    curve: Option<AssetId<Curve>>,
    #[serde(skip)]
    reactive_signal: Option<ReactiveSignalHandle>,
    #[serde(skip)]
    _phantom: PhantomData<R>,
}

/// Support for old format, can be remove when everything has been migrated
pub fn deserialize_static<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Variant {
        Old(Vec<f32>),
        New(f32),
    }
    let v = Variant::deserialize(deserializer)?;
    Ok(match v {
        Variant::Old(value) => value.into_iter().next().unwrap_or(1.0),
        Variant::New(value) => value,
    })
}
/// Support for old format, can be remove when everything has been migrated
pub fn deserialize_curve<'de, D>(deserializer: D) -> Result<Option<AssetId<Curve>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Variant {
        Old(Vec<AssetId<Curve>>),
        New(Option<AssetId<Curve>>),
    }
    let v = Variant::deserialize(deserializer)?;
    Ok(match v {
        Variant::Old(value) => value.into_iter().next(),
        Variant::New(value) => value,
    })
}

impl<R: Range> MultipliedCurve<R> {
    pub const fn new_multiplier(multiplier: f32) -> Self {
        Self {
            multiplier,
            curve: None,
            reactive_signal: None,
            _phantom: PhantomData,
        }
    }

    pub const fn new_curve(bytes: [u8; 16]) -> Self {
        Self {
            multiplier: 1.0,
            curve: Some(AssetId::from_uuid(Uuid::from_bytes(bytes))),
            reactive_signal: None,
            _phantom: PhantomData,
        }
    }

    pub fn multiplier(&self) -> f32 {
        self.multiplier
    }
}

pub trait Range: Eq {
    const MAX: f32;
    fn format(num: f32) -> String;
}

impl<R: Range> MultipliedCurve<R> {
    pub fn value(&self, beat_progression: f32) -> f32 {
        self.curve
            .and_then(Asset::get)
            .map(|curve| curve.data.value(beat_progression % 4.0))
            .unwrap_or(1.0)
            * self.multiplier
            * R::MAX
            //+ crate::audio::adsr_editor::ADSR_VALUE.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash)]
pub struct RangePercentage;
impl Range for RangePercentage {
    const MAX: f32 = 1.0;
    fn format(num: f32) -> String {
        format!("{:.1}%", num * 100.0)
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash)]
pub struct RangeDegrees;
impl Range for RangeDegrees {
    const MAX: f32 = 360.0;
    fn format(num: f32) -> String {
        format!("{num:.0}°")
    }
}

impl<R: Range> Default for MultipliedCurve<R> {
    fn default() -> Self {
        Self::new_multiplier(1.0)
    }
}

impl<R: Range> Eq for MultipliedCurve<R> {}

impl<R: Range> MultipliedCurve<R> {
    pub fn change_button(&mut self, ui: &mut egui::Ui, beat_progression: f32) -> bool {
        let mut changed = false;

        let mut rect = ui.available_rect_before_wrap();
        rect.set_height(20.0);
        let (left, right) = rect.split_left_right_at_fraction(0.5);
        ui.horizontal(|ui| {
            changed |= ui
                .put(
                    left.shrink(2.0),
                    GledSlider {
                        real_value: self
                            .curve
                            .and_then(Asset::get)
                            .map(|curve| curve.data.value(beat_progression % 4.0))
                            .unwrap_or(1.0)
                            * self.multiplier,
                        value: &mut self.multiplier,
                        size: left.height(),
                        horizontal: true,
                        show_label: false,
                        max_value: R::MAX,
                    },
                )
                .changed();

            ui.scope(|ui| {
                ui.set_max_width(right.shrink(2.0).width());
                let rect = MenuButton::new(if self.curve.is_some() {
                    "                        "
                } else {
                    "Dynamic"
                })
                .config(
                    MenuConfig::new().close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside),
                )
                .ui(ui, |ui| {
                    ui.set_min_width(300.0);
                    if self.curve.is_some()
                        && ui
                            .vertical_centered_justified(|ui| {
                                ui.add(Button::new("Remove Curve").fill(Color32::DARK_RED))
                            })
                            .inner
                            .clicked()
                    {
                        self.curve = None;
                        changed = true;
                        ui.close_kind(UiKind::Menu);
                    };

                    ui.set_min_height(400.0);
                    if let Some(id) =
                        AssetTree::show_asset_selection(ui, ui.make_persistent_id(Curve::NAME))
                    {
                        self.curve = Some(id);
                        ui.data_mut(|d| {
                            d.remove::<TreeViewState<usize>>(ui.make_persistent_id(Curve::NAME))
                        });
                        ui.close_kind(UiKind::Menu);
                        changed = true;
                    }
                })
                .0
                .rect;
                if let Some(curve) = self.curve.and_then(Asset::get) {
                    curve
                        .data
                        .clone()
                        .draw(ui, false, Some(beat_progression), rect);
                }
            });
        });

        changed
    }
}
