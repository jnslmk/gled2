use super::{App, PersistantState};
use crate::{
    storage::{branches, polynomials_fitting, staged_changes, Action, Branches},
    temperature::temperature,
};
use egui::{Button, ComboBox, Context, Label, Layout, Margin, Spinner, ViewportId};
use egui_flex::{item, Flex};

impl App {
    pub fn status_bar(&mut self, ctx: &Context, viewport_id: Option<ViewportId>) {
        egui::TopBottomPanel::bottom(format!("{viewport_id:?} status bar")).show(ctx, |ui| {
            egui::Frame::none()
                .inner_margin(Margin::from(1.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if let Some(framerate) = self.timing.framerate() {
                            ui.add(Label::new(format!("{framerate:.0} fps")));
                        }

                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add(Label::new(temperature()));
                            ui.add_space(4.0);
                            ui.menu_button("", |ui| {
                                ui.heading("Assets  Settings");

                                ui.label("Repository URL:");
                                ui.text_edit_singleline(&mut self.new_git_url);
                                let mut close_menu = false;
                                Flex::horizontal().show(ui, |flex| {
                                    if flex
                                        .add(item().grow(1.0), Button::new("Apply"))
                                        .inner
                                        .clicked()
                                    {
                                        close_menu = true;

                                        let mut persistant = PersistantState::get();
                                        persistant.git_url = self.new_git_url.clone();
                                        persistant.save();

                                        Action::Restart.enqueue();
                                    }
                                    if flex
                                        .add(item().grow(1.0), Button::new("Reset"))
                                        .inner
                                        .clicked()
                                    {
                                        self.new_git_url = PersistantState::git_url();
                                    }
                                });

                                if let Some(Branches {
                                    available,
                                    mut current,
                                }) = branches()
                                {
                                    ui.separator();

                                    ComboBox::new("branch", "Branch")
                                        .selected_text(&current)
                                        .show_ui(ui, |ui| {
                                            for branch in available {
                                                if ui
                                                    .selectable_value(
                                                        &mut current,
                                                        branch.clone(),
                                                        branch.clone(),
                                                    )
                                                    .changed()
                                                {
                                                    ui.close_menu();
                                                    close_menu = true;
                                                    Action::SwitchBranch(branch).enqueue();
                                                };
                                            }
                                        });
                                }

                                let staged_changes = staged_changes();
                                if staged_changes > 0 {
                                    ui.separator();

                                    ui.label("Commit message:");
                                    ui.text_edit_singleline(&mut self.commit_message);
                                    if ui
                                        .button(if staged_changes == 1 {
                                            "Commit and push change".to_string()
                                        } else {
                                            format!("Commit and push {staged_changes} changes")
                                        })
                                        .clicked()
                                    {
                                        ui.close_menu();
                                        Action::CommitAndPush {
                                            message: self.commit_message.clone(),
                                        }
                                        .enqueue();
                                        self.commit_message = String::new();
                                    }
                                }

                                if close_menu {
                                    ui.close_menu();
                                }
                            });
                            ui.add_space(4.0);
                            match polynomials_fitting() {
                                0 => (),
                                n => {
                                    ui.add(Spinner::new()).on_hover_ui(|ui| {
                                        ui.label(if n == 1 {
                                            "Fitting one polynomial".to_owned()
                                        } else {
                                            format!("Fitting {n} polynomials")
                                        });
                                    });
                                    ui.add_space(4.0);
                                }
                            }
                        });
                    });
                });
        });
    }
}
