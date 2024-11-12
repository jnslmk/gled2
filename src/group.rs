use crate::{app::svg::groups, ui::ChangeButton};
use egui::{Button, Color32, RichText, Stroke};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq, Clone, PartialOrd, Ord)]
#[serde(transparent)]
pub struct Group(String);

impl From<&Group> for String {
    fn from(val: &Group) -> Self {
        val.0.clone()
    }
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq, Clone)]
pub enum Groups {
    #[default]
    None,
    One(Group),
    Two(Group, Group),
}

impl Groups {
    pub fn get(&self, use_secondary_group: bool) -> Option<&Group> {
        match self {
            Groups::None => None,
            Groups::One(group) => Some(group),
            Groups::Two(primary, secondary) => {
                if use_secondary_group {
                    Some(secondary)
                } else {
                    Some(primary)
                }
            }
        }
    }
}

impl ChangeButton for Groups {
    fn change_button(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        ui.scope(|ui| {
            ui.horizontal_wrapped(|ui| {
                for group in groups() {
                    if ui
                        .add(group_button(
                            &group,
                            match (&self, &group) {
                                (Groups::None, _) => false,
                                (Groups::One(g), _) => g == &group,
                                (Groups::Two(..), _) => todo!(),
                            },
                        ))
                        .clicked()
                    {
                        //TODO: Support multiple groups
                        *self = Self::One(group);
                        changed = true;
                    };
                }
            })
        });
        changed
    }
}

pub fn group_button(group: &Group, selected: bool) -> Button {
    static COLORS: &[Color32] = &[
        Color32::from_rgb(175, 213, 129),
        Color32::from_rgb(177, 152, 221),
        Color32::from_rgb(140, 181, 255),
        Color32::from_rgb(217, 176, 140),
        Color32::from_rgb(217, 148, 140),
        Color32::from_rgb(113, 208, 132),
        Color32::from_rgb(213, 129, 192),
        Color32::from_rgb(163, 218, 224),
        Color32::from_rgb(222, 233, 190),
        Color32::from_rgb(255, 255, 255),
    ];

    let index = groups().iter().position(|g| g == group).unwrap_or_default();
    let button = Button::new(RichText::new(group).color(Color32::from_black_alpha(200)))
        .fill(COLORS[index % COLORS.len()]);
    if selected {
        button.stroke(Stroke::new(3.0, Color32::RED))
    } else {
        button
    }
}
