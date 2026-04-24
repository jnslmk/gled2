use super::Curve;
use crate::{
    audio::{sound_data::SoundData, sound_trigger_editor::SoundTriggerEditor},
    storage::{Asset, AssetId, AssetTrait, collections::Collections},
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
    #[serde(default = "default_multiplier")]
    pub multiplier: f32,
    curve: Option<AssetId<Curve>>,
    #[serde(default)]
    sound_trigger: Option<SoundTriggerEditor>,
    #[serde(skip)]
    _phantom: PhantomData<R>,
}

fn default_multiplier() -> f32 {
    1.0
}

impl<R: Range> MultipliedCurve<R> {
    pub const fn new_multiplier(multiplier: f32) -> Self {
        Self {
            multiplier,
            curve: None,
            sound_trigger: None,
            _phantom: PhantomData,
        }
    }

    pub fn set_multiplier(&mut self, multiplier: f32) {
        self.multiplier = multiplier;
    }

    pub fn set_curve(&mut self, curve: Option<AssetId<Curve>>) {
        self.curve = curve;
    }

    pub fn curve(&self) -> Option<AssetId<Curve>> {
        self.curve
    }

    pub const fn new_curve(bytes: [u8; 16]) -> Self {
        Self {
            multiplier: 1.0,
            curve: Some(AssetId::from_uuid(Uuid::from_bytes(bytes))),
            sound_trigger: None,
            _phantom: PhantomData,
        }
    }
}

pub trait Range: Eq {
    const MAX: f32;
    fn format(num: f32) -> String;
}

impl<R: Range> MultipliedCurve<R> {
    pub fn value(
        &self,
        beat_progression: f32,
        collections: &Collections,
        sound_data: &SoundData,
    ) -> f32 {
        self.curve
            .and_then(|id| Asset::get(id, collections))
            .map(|curve| curve.data.value(beat_progression % 4.0))
            .unwrap_or(1.0)
            * self.multiplier
            * R::MAX
            * self.sound_trigger_value(sound_data)
    }

    pub fn sound_trigger_value(&self, sound_data: &SoundData) -> f32 {
        self.sound_trigger
            .as_ref()
            .map(|editor| editor.sound_trigger_handle.level(sound_data))
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
    pub fn change_button(
        &mut self,
        ui: &mut egui::Ui,
        beat_progression: f32,
        collections: &mut Collections,
        sound_data: &mut SoundData,
    ) -> bool {
        let mut changed = false;

        let mut rect = ui.available_rect_before_wrap();
        rect.set_height(20.0);
        let (left, right) = rect.split_left_right_at_fraction(0.5);
        ui.horizontal(|ui| {
            let preview_value = self
                .curve
                .and_then(|curve_id| Asset::get(curve_id, collections))
                .map(|curve| curve.data.value(beat_progression % 4.0))
                .unwrap_or(1.0)
                * self.multiplier
                * self.sound_trigger_value(sound_data);
            changed |= ui
                .put(
                    left,
                    GledSlider::new(&mut self.multiplier, R::MAX)
                        .horizontal()
                        .preview_value(preview_value)
                        .size(left.height()),
                )
                .changed();

            let (mut curve_rect, mut sound_trigger_rect) = right
                .shrink2(vec2(2.0, 0.0))
                .split_left_right_at_fraction(0.5);
            curve_rect = curve_rect.shrink2(vec2(2.0, 0.0));
            sound_trigger_rect = sound_trigger_rect.shrink2(vec2(2.0, 0.0));
            ui.scope_builder(UiBuilder::default().max_rect(curve_rect), |ui| {
                ui.take_available_space();
                let rect = MenuButton::from_button(
                    Button::new(if self.curve.is_some() {
                        RichText::default()
                    } else {
                        icons::CHART_LINE.regular()
                    })
                    .min_size(sound_trigger_rect.size()),
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
                    if let Some(id) = AssetTree::show_asset_selection(
                        ui,
                        ui.make_persistent_id(Curve::NAME),
                        collections,
                    ) {
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
                if let Some(curve) = self.curve.and_then(|id| Asset::get(id, collections)) {
                    curve
                        .data
                        .clone()
                        .draw_curve(ui, false, Some(beat_progression), rect);
                }
            });

            // Menu for sound trigger
            ui.scope_builder(UiBuilder::default().max_rect(sound_trigger_rect), |ui| {
                ui.take_available_space();
                let rect = MenuButton::from_button(
                    Button::new(if self.sound_trigger.is_some() {
                        RichText::default()
                    } else {
                        icons::MICROPHONE.regular()
                    })
                    .min_size(sound_trigger_rect.size()),
                )
                .config(
                    MenuConfig::new().close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside),
                )
                .ui(ui, |ui| {
                    ui.set_min_width(300.0);
                    // Remove Button
                    if self.sound_trigger.is_some()
                        && ui
                            .vertical_centered_justified(|ui| {
                                ui.add(Button::new("Remove Trigger").fill(Color32::DARK_RED))
                            })
                            .inner
                            .clicked()
                    {
                        // this will automatically remove the sound trigger from the audio thread
                        self.sound_trigger = None;
                        changed = true;
                        ui.close_kind(UiKind::Menu);
                        return;
                    };

                    ui.set_min_height(400.0);
                    // create a sound trigger as clicking the menu button is interpreted as adding trigger behavior
                    if self.sound_trigger.is_none() {
                        self.sound_trigger = Some(SoundTriggerEditor::default());
                    }
                    // show controls for the sound trigger
                    if let Some(editor) = &mut self.sound_trigger {
                        editor.show(ui, sound_data);
                    }
                })
                .0
                .rect;
                if let Some(editor) = &mut self.sound_trigger {
                    editor.show_minified(ui, rect, sound_data);
                }
            });
        });

        changed
    }
}
