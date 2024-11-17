mod variables;

use super::{config::FloatValue::F32, AnimationConfig};
use crate::ui::ChangeButton;
use egui::{
    load::SizedTexture, Button, Color32, ComboBox, CursorIcon, Image, Layout, Sense, TextureId, Ui,
    Vec2,
};
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumIter, IntoEnumIterator};

pub use variables::{Variables, VariablesCount};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Argument {
    pub name: String,
    pub kind: ArgumentKind,
}

impl Default for Argument {
    fn default() -> Self {
        Self {
            name: "New Argument".to_string(),
            kind: ArgumentKind::default(),
        }
    }
}

impl Argument {
    pub fn function_name(&self) -> String {
        self.name.to_ascii_lowercase().replace(" ", "_")
    }

    pub fn function(&self) -> String {
        self.kind.variables().function(&self.function_name())
    }

    pub fn shader_code_for_getter(&self, count: VariablesCount) -> String {
        self.kind
            .variables()
            .shader_code_for_getter(&self.function_name(), count)
    }

    pub fn change_ui(&mut self, ui: &mut Ui, index: usize, remove: &mut Option<usize>) -> bool {
        let mut changed = false;

        ui.horizontal(|ui| {
            ui.label("Name");
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(Button::new("🗑 Remove Argument").fill(Color32::DARK_RED))
                    .clicked()
                {
                    *remove = Some(index);
                }
            });
        });
        ui.vertical_centered_justified(|ui| {
            changed |= ui.text_edit_singleline(&mut self.name).changed();
        });

        ui.label("Generated Function");
        ui.vertical_centered_justified(|ui| {
            ui.add_enabled_ui(false, |ui| ui.text_edit_singleline(&mut self.function()));
        });

        ui.label("Kind");
        ComboBox::from_id_salt(format!("AnimationKind:{index}"))
            .selected_text(ArgumentKindId::from(&self.kind).as_ref())
            .show_ui(ui, |ui| {
                let mut id = ArgumentKindId::from(&self.kind);
                for new_id in ArgumentKindId::iter() {
                    if ui
                        .selectable_value(&mut id, new_id, new_id.as_ref())
                        .clicked()
                    {
                        self.kind = ArgumentKind::from(new_id);
                        changed = true;
                    }
                }
            });

        match &mut self.kind {
            ArgumentKind::Center
            | ArgumentKind::Percentage
            | ArgumentKind::Degrees
            | ArgumentKind::Checkbox => (),
            ArgumentKind::Selection { variants } => {
                ui.label("Variants");
                ui.vertical_centered_justified(|ui| {
                    if ui.button("+ Add Variant").clicked() {
                        variants.push("New Variant".to_string());
                        changed = true;
                    }
                });
                let mut remove = None;
                for (index, variant) in variants.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        if ui.add(Button::new("🗑").fill(Color32::DARK_RED)).clicked() {
                            remove = Some(index);
                        }
                        ui.label(format!("{index}"));
                        changed |= ui.text_edit_singleline(variant).changed();
                    });
                }
                if let Some(index) = remove {
                    variants.remove(index);
                    changed = true;
                }
            }
            ArgumentKind::Slider { min, max } => {
                ui.label("Min");
                ui.vertical_centered_justified(|ui| {
                    changed |= ui.add(egui::DragValue::new(min)).changed();
                });

                ui.label("Max");
                ui.vertical_centered_justified(|ui| {
                    changed |= ui.add(egui::DragValue::new(max)).changed();
                });
            }
        }

        changed
    }

    pub fn config_ui(
        &self,
        config: &mut AnimationConfig,
        ui: &mut Ui,
        count: VariablesCount,
        rendered: TextureId,
    ) -> bool {
        let mut changed = false;
        ui.label(&self.name);
        match &self.kind {
            ArgumentKind::Center => {
                ui.vertical_centered_justified(|ui| {
                    ui.menu_button("Select center of animation", |ui| {
                        let size = 300.0;
                        let res = ui.add(
                            Image::new(SizedTexture::new(rendered, Vec2::splat(size)))
                                .sense(Sense::click()),
                        );
                        if let Some(pos) =
                            res.hover_pos().map(|pos| pos - res.rect.min).filter(|pos| {
                                pos.x > 0.0 || pos.y > 0.0 || pos.x < size || pos.y < size
                            })
                        {
                            ui.output_mut(|o| o.cursor_icon = CursorIcon::Crosshair);
                            if let Some(float) = config.float(count.f32) {
                                *float = F32(pos.x / size);
                            }
                            if let Some(float) = config.float(count.f32 + 1) {
                                *float = F32(1.0 - pos.y / size);
                            }
                            changed = true;
                        }
                        if res.clicked() {
                            ui.close_menu();
                        }
                    });
                });
            }
            ArgumentKind::Selection { variants } => {
                let Some(value) = config.u32(count.u32) else {
                    return false;
                };
                ui.horizontal(|ui| {
                    for (i, variant) in variants.iter().enumerate() {
                        if ui.radio_value(value, i as u32, variant).changed() {
                            changed = true;
                        }
                    }
                });
            }
            ArgumentKind::Percentage => {
                let Some(value) = config.float(count.f32) else {
                    return false;
                };

                ui.vertical_centered_justified(|ui| {
                    changed |= value.percentage().change_button(ui);
                });
            }
            ArgumentKind::Degrees => {
                let Some(value) = config.float(count.f32) else {
                    return false;
                };

                ui.vertical_centered_justified(|ui| {
                    changed |= value.degrees().change_button(ui);
                });
            }
            ArgumentKind::Slider { min, max } => {
                let Some(value) = config.u32(count.u32) else {
                    return false;
                };

                ui.vertical_centered_justified(|ui| {
                    changed |= ui.add(egui::Slider::new(value, *min..=*max)).changed();
                });
            }
            ArgumentKind::Checkbox => {
                let Some(value) = config.u32(count.u32) else {
                    return false;
                };

                let mut bool_value = *value != 0;

                if ui.checkbox(&mut bool_value, "").changed() {
                    *value = bool_value as u32;
                    changed = true;
                }
            }
        }
        changed
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ArgumentKind {
    Center,
    Selection {
        variants: Vec<String>,
    },
    Slider {
        min: u32,
        max: u32,
    },
    Checkbox,
    #[default]
    Percentage,
    Degrees,
}

impl ArgumentKind {
    pub fn variables(&self) -> Variables {
        match self {
            Self::Center => Variables::Vec2F32,
            Self::Selection { .. } => Variables::U32,
            Self::Slider { .. } => Variables::U32,
            Self::Checkbox => Variables::U32,
            Self::Percentage => Variables::F32,
            Self::Degrees => Variables::F32,
        }
    }
}

#[derive(
    Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, AsRefStr, EnumIter,
)]
pub enum ArgumentKindId {
    Center,
    Selection,
    Slider,
    Checkbox,
    #[default]
    Percentage,
    Degrees,
}

impl From<&ArgumentKind> for ArgumentKindId {
    fn from(kind: &ArgumentKind) -> Self {
        match kind {
            ArgumentKind::Center => Self::Center,
            ArgumentKind::Selection { .. } => Self::Selection,
            ArgumentKind::Slider { .. } => Self::Slider,
            ArgumentKind::Checkbox => Self::Checkbox,
            ArgumentKind::Percentage => Self::Percentage,
            ArgumentKind::Degrees => Self::Degrees,
        }
    }
}

impl From<ArgumentKindId> for ArgumentKind {
    fn from(kind: ArgumentKindId) -> Self {
        match kind {
            ArgumentKindId::Center => Self::Center,
            ArgumentKindId::Selection => Self::Selection { variants: vec![] },
            ArgumentKindId::Slider => Self::Slider { min: 0, max: 100 },
            ArgumentKindId::Checkbox => Self::Checkbox,
            ArgumentKindId::Percentage => Self::Percentage,
            ArgumentKindId::Degrees => Self::Degrees,
        }
    }
}
