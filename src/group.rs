use crate::{app::svg::groups, ui::ChangeButton};
use egui::{Button, Color32, RichText, Stroke, Ui};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
};

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

pub type Groups = BTreeMap<usize, Group>;
pub type GroupIndices = BTreeSet<usize>;

impl ChangeButton for Groups {
    fn change_button(&mut self, ui: &mut egui::Ui) -> bool {
        if groups().is_empty() {
            ui.label("No groups available");
            return false;
        }

        let mut changed = false;
        let mut next_index = 0;
        while self.contains_key(&next_index) {
            next_index += 1;
        }
        let mut insert = None;
        let mut remove = None;
        for (index, group) in self.iter() {
            ui.scope(|ui| {
                {
                    let widgets = &mut ui.visuals_mut().widgets;
                    widgets.inactive.fg_stroke.color = Color32::from_black_alpha(200);
                    widgets.inactive.weak_bg_fill = color(group);
                    widgets.hovered.fg_stroke.color = Color32::from_black_alpha(200);
                    widgets.hovered.weak_bg_fill = color(group);
                }
                ui.menu_button(format!("{index} {}", group.0), |ui| {
                    ui.set_min_width(200.0);
                    if ui
                        .add(Button::new("🗑 Remove").fill(Color32::DARK_RED))
                        .clicked()
                    {
                        remove = Some(*index);
                        changed = true;
                    }
                    let mut group = Some(group.to_owned());
                    if group_buttons(ui, &mut group) {
                        ui.close_menu();
                        if let Some(group) = group {
                            insert = Some((*index, group));
                        } else {
                            remove = Some(*index);
                        }
                        changed = true;
                    }
                });
            });
        }

        let mut new_group = None;
        ui.menu_button(format!("+ Add Group {next_index}"), |ui| {
            ui.set_min_width(200.0);
            if group_buttons(ui, &mut new_group) {
                ui.close_menu();
                if let Some(group) = new_group {
                    insert = Some((next_index, group));
                    changed = true;
                }
            }
        });

        if let Some((index, group)) = insert {
            self.insert(index, group);
        }
        if let Some(index) = remove {
            self.remove(&index);
        }

        changed
    }
}

fn group_buttons(ui: &mut Ui, selected: &mut Option<Group>) -> bool {
    let mut changed = false;
    for group in groups() {
        if ui
            .add(group_button(&group, selected.as_ref() == Some(&group)))
            .clicked()
        {
            *selected = Some(group);
            changed = true;
        }
    }
    changed
}

pub fn group_button(group: &Group, selected: bool) -> Button {
    let button =
        Button::new(RichText::new(group).color(Color32::from_black_alpha(200))).fill(color(group));

    if selected {
        button.stroke(Stroke::new(3.0, Color32::RED))
    } else {
        button
    }
}

fn color(group: &Group) -> Color32 {
    let index = groups().iter().position(|g| g == group).unwrap_or_default();
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
    COLORS[index % COLORS.len()]
}
