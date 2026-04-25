use super::Curve;
use crate::{
    audio::{sound_data::SoundData, sound_trigger_editor::SoundTriggerEditor},
    storage::{Asset, AssetId, AssetTrait, collections::Collections},
    ui::{asset_tree::AssetTree, gled_slider::GledSlider},
};
use egui::{
    Button, Color32, RichText, Sense, Stroke, UiBuilder, UiKind,
    containers::menu::{MenuButton, MenuConfig},
};
use egui_ltreeview::TreeViewState;
use egui_phosphor_icons::icons;
use emath::{Pos2, Rect, Vec2, vec2};
use serde::{Deserialize, Serialize};
use std::{f32::consts::TAU, marker::PhantomData};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MultipliedCurve<R> {
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

impl<R> MultipliedCurve<R> {
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

    pub fn sound_trigger_value(&self, sound_data: &SoundData) -> f32 {
        self.sound_trigger
            .as_ref()
            .map(|editor| editor.sound_trigger_handle.level(sound_data))
            .unwrap_or(1.0)
    }

    fn curve_preview_value(&self, beat_progression: f32, collections: &Collections) -> f32 {
        self.curve
            .and_then(|id| Asset::get(id, collections))
            .map(|curve| curve.data.value(beat_progression % 4.0))
            .unwrap_or(1.0)
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash)]
pub struct RangePercentage;

#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash)]
pub struct RangeDegrees;

impl<R> Default for MultipliedCurve<R> {
    fn default() -> Self {
        Self::new_multiplier(default_multiplier())
    }
}

impl<R> Eq for MultipliedCurve<R> {}

impl<R> PartialEq for MultipliedCurve<R> {
    fn eq(&self, other: &Self) -> bool {
        self.multiplier == other.multiplier
            && self.curve == other.curve
            && self.sound_trigger == other.sound_trigger
    }
}

impl MultipliedCurve<RangePercentage> {
    pub fn value(
        &self,
        beat_progression: f32,
        collections: &Collections,
        sound_data: &SoundData,
    ) -> f32 {
        self.curve_preview_value(beat_progression, collections)
            * self.multiplier
            * self.sound_trigger_value(sound_data)
    }

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
            let preview_value = self.curve_preview_value(beat_progression, collections)
                * self.multiplier
                * self.sound_trigger_value(sound_data);
            changed |= ui
                .put(
                    left,
                    GledSlider::new(&mut self.multiplier, 1.0)
                        .horizontal()
                        .preview_value(preview_value)
                        .size(left.height()),
                )
                .changed();

            changed |= shared_curve_and_trigger_menus(
                ui,
                right,
                &mut self.curve,
                &mut self.sound_trigger,
                beat_progression,
                collections,
                sound_data,
            );
        });

        changed
    }
}

impl MultipliedCurve<RangeDegrees> {
    pub fn value(
        &self,
        beat_progression: f32,
        collections: &Collections,
        sound_data: &SoundData,
    ) -> f32 {
        self.curve_preview_value(beat_progression, collections)
            * self.multiplier
            * 360.0
            * self.sound_trigger_value(sound_data)
    }

    pub fn change_button(
        &mut self,
        ui: &mut egui::Ui,
        beat_progression: f32,
        collections: &mut Collections,
        sound_data: &mut SoundData,
    ) -> bool {
        let mut changed = false;

        let mut rect = ui.available_rect_before_wrap();
        rect.set_height(40.0);
        let (left, right) = rect.split_left_right_at_fraction(0.5);
        ui.horizontal(|ui| {
            let preview_value = self.curve_preview_value(beat_progression, collections)
                * self.multiplier
                * self.sound_trigger_value(sound_data);
            changed |= angle_dial(ui, left, &mut self.multiplier, preview_value);

            changed |= shared_curve_and_trigger_menus(
                ui,
                right,
                &mut self.curve,
                &mut self.sound_trigger,
                beat_progression,
                collections,
                sound_data,
            );
        });

        changed
    }
}

/// Curve picker + sound trigger menu shared by all `MultipliedCurve<R>` UIs.
fn shared_curve_and_trigger_menus(
    ui: &mut egui::Ui,
    right: Rect,
    curve_id: &mut Option<AssetId<Curve>>,
    sound_trigger: &mut Option<SoundTriggerEditor>,
    beat_progression: f32,
    collections: &mut Collections,
    sound_data: &mut SoundData,
) -> bool {
    let mut changed = false;

    let (mut curve_rect, mut sound_trigger_rect) = right
        .shrink2(vec2(2.0, 0.0))
        .split_left_right_at_fraction(0.5);
    curve_rect = curve_rect.shrink2(vec2(2.0, 0.0));
    sound_trigger_rect = sound_trigger_rect.shrink2(vec2(2.0, 0.0));

    ui.scope_builder(UiBuilder::default().max_rect(curve_rect), |ui| {
        ui.take_available_space();
        let rect = MenuButton::from_button(
            Button::new(if curve_id.is_some() {
                RichText::default()
            } else {
                icons::CHART_LINE.regular()
            })
            .min_size(sound_trigger_rect.size()),
        )
        .config(MenuConfig::new().close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside))
        .ui(ui, |ui| {
            ui.set_min_width(300.0);
            if curve_id.is_some()
                && ui
                    .vertical_centered_justified(|ui| {
                        ui.add(Button::new("Remove Curve").fill(Color32::DARK_RED))
                    })
                    .inner
                    .clicked()
            {
                *curve_id = None;
                changed = true;
                ui.close_kind(UiKind::Menu);
            };

            ui.set_min_height(400.0);
            if let Some(id) = AssetTree::show_asset_selection(
                ui,
                ui.make_persistent_id(Curve::NAME),
                collections,
            ) {
                *curve_id = Some(id);
                ui.data_mut(|d| {
                    d.remove::<TreeViewState<usize>>(ui.make_persistent_id(Curve::NAME))
                });
                ui.close_kind(UiKind::Menu);
                changed = true;
            }
        })
        .0
        .rect;
        if let Some(curve) = curve_id.and_then(|id| Asset::get(id, collections)) {
            curve
                .data
                .clone()
                .draw_curve(ui, false, Some(beat_progression), rect);
        }
    });

    ui.scope_builder(UiBuilder::default().max_rect(sound_trigger_rect), |ui| {
        ui.take_available_space();
        let rect = MenuButton::from_button(
            Button::new(if sound_trigger.is_some() {
                RichText::default()
            } else {
                icons::MICROPHONE.regular()
            })
            .min_size(sound_trigger_rect.size()),
        )
        .config(MenuConfig::new().close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside))
        .ui(ui, |ui| {
            ui.set_min_width(300.0);
            if sound_trigger.is_some()
                && ui
                    .vertical_centered_justified(|ui| {
                        ui.add(Button::new("Remove Trigger").fill(Color32::DARK_RED))
                    })
                    .inner
                    .clicked()
            {
                *sound_trigger = None;
                changed = true;
                ui.close_kind(UiKind::Menu);
                return;
            };

            ui.set_min_height(400.0);
            if sound_trigger.is_none() {
                *sound_trigger = Some(SoundTriggerEditor::default());
            }
            if let Some(editor) = sound_trigger {
                editor.show(ui, sound_data);
            }
        })
        .0
        .rect;
        if let Some(editor) = sound_trigger {
            editor.show_minified(ui, rect, sound_data);
        }
    });

    changed
}

/// Clock-style circular angle picker. `value` is in `[0, 1)` representing one full turn.
/// 0 corresponds to 12 o'clock and increases clockwise. Snaps to the nearest quarter
/// (0/90/180/270°) when the pointer is near the outer edge of the circle.
fn angle_dial(ui: &mut egui::Ui, area: Rect, value: &mut f32, preview_value: f32) -> bool {
    const TOP_COLOR: Color32 = Color32::from_rgb(53, 53, 49);
    const RING_COLOR: Color32 = Color32::from_rgb(103, 103, 94);
    const HANDLE_COLOR: Color32 = Color32::from_rgb(59, 255, 0);
    const PREVIEW_COLOR: Color32 = Color32::from_rgb(255, 176, 100);
    const SNAP_RADIUS_FRACTION: f32 = 0.85;

    let side = area.width().min(area.height());
    let center = area.center();
    let dial_rect = Rect::from_center_size(center, Vec2::splat(side));
    let radius = side * 0.5;

    let response = ui.allocate_rect(dial_rect, Sense::click_and_drag());
    let mut changed = false;

    if let Some(pointer) = response.interact_pointer_pos()
        && (response.dragged() || response.clicked())
    {
        let delta = pointer - center;
        let r_norm = (delta.length() / radius).clamp(0.0, 1.0);
        // 0° at top, increasing clockwise.
        let mut angle = delta.y.atan2(delta.x) + std::f32::consts::FRAC_PI_2;
        if angle < 0.0 {
            angle += TAU;
        }
        let mut new_value = angle / TAU;
        if r_norm > SNAP_RADIUS_FRACTION {
            new_value = (new_value * 4.0).round() / 4.0;
            if new_value >= 1.0 {
                new_value = 0.0;
            }
        }
        new_value = new_value.clamp(0.0, 1.0);
        if (new_value - *value).abs() > f32::EPSILON {
            *value = new_value;
            changed = true;
        }
    }

    let painter = ui.painter_at(dial_rect);
    painter.circle_filled(center, radius, TOP_COLOR);
    painter.circle_stroke(center, radius, Stroke::new(1.0, RING_COLOR));

    // Quarter tick marks at 12/3/6/9.
    for i in 0..4 {
        let a = (i as f32) * std::f32::consts::FRAC_PI_2 - std::f32::consts::FRAC_PI_2;
        let inner = center + Vec2::new(a.cos(), a.sin()) * (radius * 0.8);
        let outer = center + Vec2::new(a.cos(), a.sin()) * radius;
        painter.line_segment([inner, outer], Stroke::new(1.0, RING_COLOR));
    }

    let draw_hand = |val: f32, color: Color32, width: f32| {
        let angle = val * TAU - std::f32::consts::FRAC_PI_2;
        let tip = center + Vec2::new(angle.cos(), angle.sin()) * (radius - 1.0);
        painter.line_segment([center, tip], Stroke::new(width, color));
    };

    if (preview_value - *value).abs() > f32::EPSILON {
        draw_hand(preview_value, PREVIEW_COLOR, 1.5);
    }
    draw_hand(*value, HANDLE_COLOR, 2.0);

    // Angle label below the dial center.
    let label = format!("{:.0}°", *value * 360.0);
    painter.text(
        Pos2::new(center.x, area.max.y - 1.0),
        egui::Align2::CENTER_BOTTOM,
        label,
        egui::FontId::proportional(radius.min(10.0)),
        HANDLE_COLOR,
    );

    changed
}
