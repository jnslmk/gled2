use crate::{app::svg::groups, ui::ChangeButton};
use egui::{Button, Color32, RichText, Stroke};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq, Clone, PartialOrd, Ord)]
#[serde(transparent)]
pub struct Group(String);

impl From<&Group> for String {
    fn from(val: &Group) -> Self {
        val.0.clone()
    }
}

impl Display for Group {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
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
    pub fn get(&self, use_secondary: bool) -> Option<&Group> {
        match self {
            Groups::None => None,
            Groups::One(group) => Some(group),
            Groups::Two(primary, secondary) => {
                if use_secondary {
                    Some(secondary)
                } else {
                    Some(primary)
                }
            }
        }
    }

    pub fn set(&mut self, use_secondary: bool, group: Group) {
        match self {
            Groups::None => {
                *self = Groups::One(group);
            }
            Groups::One(primary) => {
                if use_secondary {
                    *self = Groups::Two(primary.clone(), group);
                } else {
                    *primary = group;
                }
            }
            Groups::Two(primary, secondary) => {
                if use_secondary {
                    *secondary = group;
                } else {
                    *primary = group;
                }
            }
        }
    }
}

impl ChangeButton for Groups {
    fn change_button(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        for (name, use_secondary) in [("Primary", false), ("Secondary", true)] {
            let group = self.get(use_secondary);
            let index = group.and_then(|group| groups().iter().position(|g| g == group));

            ui.menu_button(
                {
                    let mut title = RichText::new(if let Some(group) = group {
                        format!("{name}: {group}")
                    } else {
                        format!("Select {name}")
                    });
                    if let Some(color) = color(index) {
                        title = title
                            .color(Color32::from_black_alpha(200))
                            .background_color(color);
                    }
                    title
                },
                |ui| {
                    for group in groups() {
                        if ui
                            .add(group_button(
                                &group,
                                self.get(use_secondary) == Some(&group),
                            ))
                            .clicked()
                        {
                            self.set(use_secondary, group);
                            ui.close_menu();
                            changed = true;
                        };
                    }
                },
            );
        }

        changed
    }
}

pub fn group_button(group: &Group, selected: bool) -> Button {
    let index = groups().iter().position(|g| g == group);
    let color = color(index);
    let mut button = Button::new({
        let mut text = RichText::new(group);
        if color.is_some() {
            text = text.color(Color32::from_black_alpha(200));
        }
        text
    });
    if let Some(color) = color {
        button = button.fill(color);
    }
    if selected {
        button.stroke(Stroke::new(3.0, Color32::RED))
    } else {
        button
    }
}

fn color(index: Option<usize>) -> Option<Color32> {
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
    Some(COLORS[index? % COLORS.len()])
}
