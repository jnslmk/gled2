pub mod dnd;
pub mod widget;

use super::ChangeButton;
use crate::{
    app::timing::Timing,
    audio::sound_data::SoundData,
    pipeline::group::Groups,
    storage::{
        asset::{AssetTrait, palette::Palette, scene::instance::SceneInstance},
        collections::Collections,
    },
    ui::{asset_tree::AssetTree, effect::widget::EffectWidget},
};
use egui::{Checkbox, Margin, ScrollArea, UiKind, Vec2, scroll_area::ScrollBarVisibility::AlwaysVisible};
use egui_modal::Modal;
use egui_ltreeview::TreeViewState;

impl SceneInstance {
    pub fn config_ui(
        &mut self,
        ui: &mut egui::Ui,
        svg: Option<egui::TextureHandle>,
        groups: Groups,
        timing: &Timing,
        collections: &mut Collections,
        sound_data: &mut SoundData,
    ) {
        let mut beat_progression = timing.beat_progression();
        beat_progression +=
            self.beat_progression_offset
                .value(beat_progression, collections, sound_data);
        let width = ui.available_width() - 20.0;

        ScrollArea::vertical()
            .scroll_bar_visibility(AlwaysVisible)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.set_min_height(ui.available_height());

                ui.horizontal(|ui| {
                    egui::Frame::NONE
                        .inner_margin(Margin::from(3.0))
                        .show(ui, |ui| {
                            ui.set_max_width(width / 2.0);
                            ui.vertical(|ui| {
                                ui.label("Name");
                                ui.text_edit_singleline(&mut self.name);

                                ui.label("Activation Input");
                                self.activation_input.change_button(ui);

                                ui.label("Flash Input");
                                self.flash_input.change_button(ui);

                                if self.flash_input.is_some() {
                                    ui.label("Set Offset On Flash");
                                    ui.checkbox(&mut self.set_offset_on_flash, "");
                                }

                                ui.label("Dimmer Input");
                                self.dimmer_input.change_button(ui);

                                ui.label("Scene highlight color");
                                self.color.change_button(ui);
                            });
                        });
                    egui::Frame::NONE
                        .inner_margin(Margin::from(3.0))
                        .show(ui, |ui| {
                            ui.set_max_width(width / 2.0);
                            ui.vertical(|ui| {
                                ui.label("Active");
                                ui.add(Checkbox::new(&mut self.active, ""));

                                ui.label("Opacity");
                                ui.vertical_centered_justified(|ui| {
                                    self.opacity.change_button(
                                        ui,
                                        beat_progression,
                                        collections,
                                        sound_data,
                                    );
                                });

                                ui.label("Ignore Main Dimmer");
                                ui.add(Checkbox::new(&mut self.ignore_main_dimmer, ""));

                                ui.label("Beat offset");
                                ui.vertical_centered_justified(|ui| {
                                    self.beat_progression_offset.change_button(
                                        ui,
                                        timing.beat_progression(),
                                        collections,
                                        sound_data,
                                    );
                                });
                            });
                        });
                });

                egui::Frame::NONE
                    .inner_margin(Margin::from(6.0))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        self.groups_overwrite.change_button(ui);
                    });

                egui::Frame::NONE
                    .inner_margin(Margin::from(6.0))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        let mut overwrite = self.palette_overwrite.is_some();
                        ui.checkbox(&mut overwrite, "Overwrite Palette");
                        if self.palette_overwrite.is_none() && overwrite {
                            self.palette_overwrite = Some(None);
                        } else if self.palette_overwrite.is_some() && !overwrite {
                            self.palette_overwrite = None;
                        }

                        if let Some(palette_overwrite) = self.palette_overwrite.as_mut() {
                            let response = ui.menu_button("📂 Palette", |ui| {
                                if ui.button("No Palette").clicked() {
                                    *palette_overwrite = None;
                                    ui.close_kind(UiKind::Menu);
                                }

                                if let Some(palette_id) = AssetTree::<Palette>::show_asset_selection(
                                    ui,
                                    ui.make_persistent_id(Palette::NAME),
                                    collections,
                                ) {
                                    *palette_overwrite = crate::storage::asset::Asset::get(
                                        palette_id,
                                        collections,
                                    )
                                    .map(|palette| palette.data.clone());
                                    ui.data_mut(|data| {
                                        data.remove::<TreeViewState<usize>>(
                                            ui.make_persistent_id(Palette::NAME),
                                        )
                                    });
                                    ui.close_kind(UiKind::Menu);
                                }
                            }).response;

                            if let Some(palette) = palette_overwrite.as_ref() {
                                palette.show(ui, response.rect);
                            }
                        }
                    });

                ui.separator();

                ui.label("Animations");

                ui.horizontal_wrapped(|ui| {
                    for (index, effect) in self.scene.effects.iter_mut().enumerate() {
                        let mut selected = 0;
                        ui.add_sized(
                            Vec2::splat(100.0),
                            EffectWidget {
                                show_group: true,
                                selectable: Some((&mut selected, 1)),
                                effect,
                                svg: svg.clone(),
                                groups: Some(&groups),
                                groups_show_index: false,
                                beat_progression: Some(beat_progression),
                                collections,
                                sound_data,
                            },
                        );

                        let modal = Modal::new(ui.ctx(), format!("effect settings {index}"))
                            .with_close_on_outside_click(true);
                        let svg = svg.clone();
                        modal.show(|ui| {
                            modal.title(ui, "Overwrite Effect Settings");
                            ui.set_width(500.0);
                            ui.horizontal(|ui| {
                                ui.add_sized(
                                    Vec2::splat(100.0),
                                    EffectWidget {
                                        show_group: true,
                                        selectable: None,
                                        effect,
                                        svg: svg.clone(),
                                        groups: Some(&groups),
                                        groups_show_index: true,
                                        beat_progression: Some(beat_progression),
                                        collections,
                                        sound_data,
                                    },
                                );
                                ui.vertical(|ui| {
                                    effect.config_ui(
                                        ui,
                                        false,
                                        svg.clone(),
                                        beat_progression,
                                        collections,
                                        sound_data,
                                    );
                                });
                            });
                        });
                        if selected == 1 {
                            modal.open();
                        }
                    }
                });
            });
    }
}
