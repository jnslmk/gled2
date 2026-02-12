use super::Curve;
use crate::audio::adsr_editor::ADSR;
use crate::{
    storage::{Asset, AssetId, AssetTrait},
    ui::{asset_tree::AssetTree, gled_slider::GledSlider},
};
use egui::{
    Button, Color32, RichText, UiBuilder, UiKind,
    containers::menu::{MenuButton, MenuConfig},
};
use egui_ltreeview::TreeViewState;
use egui_phosphor_icons::icons;
use emath::vec2;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct MultipliedCurve<R: Range> {
    #[serde(
        alias = "Static",
        deserialize_with = "deserialize_static",
        default = "default_multiplier"
    )]
    pub multiplier: f32,
    #[serde(alias = "Curve", deserialize_with = "deserialize_curve", default)]
    curve: Option<AssetId<Curve>>,
    #[serde(default)]
    adsr: Option<ADSR>,
    #[serde(skip)]
    _phantom: PhantomData<R>,
}

fn default_multiplier() -> f32 {
    1.0
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
            adsr: None,
            _phantom: PhantomData,
        }
    }

    pub const fn new_curve(bytes: [u8; 16]) -> Self {
        Self {
            multiplier: 1.0,
            curve: Some(AssetId::from_uuid(Uuid::from_bytes(bytes))),
            adsr: None,
            _phantom: PhantomData,
        }
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
            * self.adsr_value()
    }

    pub fn adsr_value(&self) -> f32 {
        self.adsr
            .as_ref()
            .map(|editor| editor.reactive_signal_handle.level())
            .unwrap_or(1.0)
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
        Self::new_multiplier(default_multiplier())
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
            let preview_value = self
                .curve
                .and_then(Asset::get)
                .map(|curve| curve.data.value(beat_progression % 4.0))
                .unwrap_or(1.0)
                * self.multiplier
                * self.adsr_value();
            changed |= ui
                .put(
                    left,
                    GledSlider::new(&mut self.multiplier, R::MAX)
                        .horizontal()
                        .preview_value(preview_value)
                        .size(left.height())
                    
                )
                .changed();

            let (mut curve_rect, mut adsr_rect) = right
                .shrink2(vec2(2.0, 0.0))
                .split_left_right_at_fraction(0.5);
            curve_rect = curve_rect.shrink2(vec2(2.0, 0.0));
            adsr_rect = adsr_rect.shrink2(vec2(2.0, 0.0));
            ui.scope_builder(UiBuilder::default().max_rect(curve_rect), |ui| {
                ui.take_available_space();
                let rect = MenuButton::from_button(
                    Button::new(if self.curve.is_some() {
                        RichText::default()
                    } else {
                        icons::CHART_LINE.regular()
                    })
                    .min_size(adsr_rect.size()),
                )
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
                        .draw_curve(ui, false, Some(beat_progression), rect);
                }
            });

            // Menu for reactive sound
            ui.scope_builder(UiBuilder::default().max_rect(adsr_rect), |ui| {
                ui.take_available_space();
                let rect = MenuButton::from_button(
                    Button::new(if self.adsr.is_some() {
                        RichText::default()
                    } else {
                        icons::MICROPHONE.regular()
                    })
                    .min_size(adsr_rect.size()),
                )
                .config(
                    MenuConfig::new().close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside),
                )
                .ui(ui, |ui| {
                    ui.set_min_width(300.0);
                    // Remove Button
                    if self.adsr.is_some()
                        && ui
                            .vertical_centered_justified(|ui| {
                                ui.add(Button::new("Remove ADSR").fill(Color32::DARK_RED))
                            })
                            .inner
                            .clicked()
                    {
                        // this will automatically remove the reactive signal from the audio thread
                        self.adsr = None;
                        changed = true;
                        ui.close_kind(UiKind::Menu);
                        return;
                    };

                    ui.set_min_height(400.0);
                    // create a reactive signal as clicking the menu button is interpreted as adding reactive sound behavior
                    if self.adsr.is_none() {
                        self.adsr = Some(ADSR::default());
                    }
                    // show controls for the reactive signal
                    if let Some(editor) = &mut self.adsr {
                        editor.show(ui);
                    }
                })
                .0
                .rect;
                if let Some(editor) = &mut self.adsr {
                    editor.show_minified(ui, rect);
                }
            });
        });

        changed
    }
}
