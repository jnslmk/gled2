use egui::{Color32, WidgetText};
use serde::{Deserialize, Serialize};

use crate::ui::ChangeButton;

#[derive(Debug, Clone, Default, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneInstanceColor {
    Red,
    Green,
    Blue,
    White,
    Orange,
    Yellow,
    Purple,
    #[default]
    Pink,
    Black,
}

impl From<SceneInstanceColor> for Color32 {
    fn from(color: SceneInstanceColor) -> Self {
        match color {
            SceneInstanceColor::Red => Color32::from_rgb(139, 0, 0), // Dark Red
            SceneInstanceColor::Green => Color32::from_rgb(0, 100, 0), // Dark Green
            SceneInstanceColor::Blue => Color32::from_rgb(0, 0, 139), // Dark Blue
            SceneInstanceColor::White => Color32::from_rgb(169, 169, 169), // Dark Gray (instead of white)
            SceneInstanceColor::Orange => Color32::from_rgb(255, 140, 0),  // Dark Orange
            SceneInstanceColor::Yellow => Color32::from_rgb(204, 204, 0),  // Dark Yellow
            SceneInstanceColor::Purple => Color32::from_rgb(75, 0, 130),   // Dark Purple (Indigo)
            SceneInstanceColor::Pink => Color32::from_rgb(231, 84, 128),   // Dark Pink
            SceneInstanceColor::Black => Color32::from_rgb(0, 0, 0),       // Black
        }
    }
}

impl ChangeButton for SceneInstanceColor {
    fn change_button(&mut self, ui: &mut egui::Ui) -> bool {
        egui::ComboBox::new("scene_instance_color", "")
            .selected_text(WidgetText::Text(format!("{self:?}")).background_color(*self))
            .show_ui(ui, |ui| {
                ui.selectable_value(self, SceneInstanceColor::Red, "Red");
                ui.selectable_value(self, SceneInstanceColor::Green, "Green");
                ui.selectable_value(self, SceneInstanceColor::Blue, "Blue");
                ui.selectable_value(self, SceneInstanceColor::White, "White");
                ui.selectable_value(self, SceneInstanceColor::Orange, "Orange");
                ui.selectable_value(self, SceneInstanceColor::Yellow, "Yellow");
                ui.selectable_value(self, SceneInstanceColor::Purple, "Purple");
                ui.selectable_value(self, SceneInstanceColor::Pink, "Pink");
                ui.selectable_value(self, SceneInstanceColor::Black, "Black");
            })
            .response
            .changed()
    }
}
