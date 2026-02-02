use crate::storage::asset::scene::grid::GridLocation;
use egui::{Button, Color32, Frame, InnerResponse, KeyboardShortcut, Response, Stroke, Ui, UiBuilder};

pub mod action;
pub mod asset;
pub mod asset_tree;
pub mod effect;
pub mod gled_slider;
pub mod input;
pub mod logo;
pub mod pills;
pub mod scene_instance;
pub mod temperature;
pub mod update_check;
pub mod window_common;
pub mod windows;

pub static FRAME_STROKE: Stroke = Stroke {
    width: 0.3,
    color: Color32::WHITE,
};

pub trait ChangeButton {
    fn change_button(&mut self, ui: &mut Ui) -> bool;
}

impl ChangeButton for f32 {
    fn change_button(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;

        let response = ui.add(egui::Slider::new(self, 0.0..=1.0));
        if response.changed() {
            changed = true;
        }

        changed
    }
}

pub trait OverwriteChangeButton: Default + ChangeButton {
    const NAME: &'static str;
}

// for overwrites
impl<T: OverwriteChangeButton> ChangeButton for Option<T> {
    fn change_button(&mut self, ui: &mut Ui) -> bool {
        let mut changed = false;

        let mut overwrite = self.is_some();
        ui.checkbox(&mut overwrite, format!("Overwrite {}", T::NAME));
        if self.is_none() && overwrite {
            *self = Some(T::default());
            changed = true;
        } else if self.is_some() && !overwrite {
            *self = None;
            changed = true;
        }
        if let Some(asset_id) = self {
            ui.vertical_centered_justified(|ui| {
                changed |= asset_id.change_button(ui);
            });
        }

        changed
    }
}

pub fn scoped_frame<T>(
    ui: &mut Ui,
    ui_builder: UiBuilder,
    frame: Frame,
    add_contents: impl FnOnce(&mut Ui) -> T,
) -> InnerResponse<T> {
    ui.scope_builder(ui_builder, |ui| {
        frame
            .show(ui, |ui| {
                ui.take_available_space();
                add_contents(ui)
            })
            .inner
    })
}

#[derive(Default)]
pub struct ContextMenuBuilder{
    actions: Vec<(KeyboardShortcut, String , fn(GridLocation), fn())>
}

impl ContextMenuBuilder {
    pub fn add_action(mut self, description: &str, keyboard_shortcut: KeyboardShortcut, ctx_action: fn(GridLocation), key_action: fn()) -> Self {
        self.actions.push((keyboard_shortcut, description.to_string(), ctx_action, key_action));
        self
    }
    pub fn show(self, response: &mut Response, location: GridLocation) {
        response.context_menu(|ctx_menu_ui| {
            for (keyboard_shortcut, description, ctx_action, _) in &self.actions {
                let context_button =
                    Button::new(format!(
                        "{}\t{}",
                        description,
                        ctx_menu_ui.ctx().format_shortcut(&keyboard_shortcut)
                    ));
                if ctx_menu_ui.add(context_button).clicked() {
                    ctx_action(location);
                    return;
                }
            }
        });
        for (keyboard_shortcut, _, _, key_action) in &self.actions {
            if response.ctx.input_mut(|i| {
                i.consume_shortcut(&keyboard_shortcut)
            }) {
                key_action();
            }
        }
    }
}