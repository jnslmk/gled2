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
    ui::{
        asset_tree::AssetTree,
        scene_effect_editor::{
            SceneEffectEditorState, scene_effect_list_ui, selected_effect_editor_ui,
        },
    },
};
use egui::{Checkbox, Margin, ScrollArea, UiKind, scroll_area::ScrollBarVisibility::AlwaysVisible};
use egui_ltreeview::TreeViewState;

impl SceneInstance {
    pub fn config_ui(
        &mut self,
        ui: &mut egui::Ui,
        effect_editor: &mut SceneEffectEditorState,
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
                            let response = ui
                                .menu_button("📂 Palette", |ui| {
                                    if ui.button("No Palette").clicked() {
                                        *palette_overwrite = None;
                                        ui.close_kind(UiKind::Menu);
                                    }

                                    if let Some(palette_id) =
                                        AssetTree::<Palette>::show_asset_selection(
                                            ui,
                                            ui.make_persistent_id(Palette::NAME),
                                            collections,
                                        )
                                    {
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
                                })
                                .response;

                            if let Some(palette) = palette_overwrite.as_ref() {
                                palette.show(ui, response.rect);
                            }
                        }
                    });

                ui.separator();

                ui.label("Animations");

                egui::Frame::default()
                    .inner_margin(Margin::from(6.0))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::DARK_GRAY))
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new("Animations")
                                    .size(12.0)
                                    .color(egui::Color32::GRAY),
                            );
                            scene_effect_list_ui(
                                ui,
                                &self.scene,
                                effect_editor,
                                true,
                                svg.clone(),
                                Some(&groups),
                                false,
                                Some(beat_progression),
                                collections,
                                sound_data,
                            );

                            ui.add_space(8.0);
                            ui.separator();
                            ui.add_space(8.0);

                            ui.label(
                                egui::RichText::new("Effect Settings")
                                    .size(12.0)
                                    .color(egui::Color32::GRAY),
                            );
                            selected_effect_editor_ui(
                                ui,
                                &mut self.scene,
                                effect_editor,
                                true,
                                svg.clone(),
                                beat_progression,
                                collections,
                                sound_data,
                            );
                        });
                    });
            });
    }
}
