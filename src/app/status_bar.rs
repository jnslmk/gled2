use super::{App, PersistantState};
use crate::{
    storage::{branches, polynomials_fitting, staged_files, working, Action, Branches},
    temperature::temperature,
};
use egui::{
    Button, Color32, ComboBox, Context, Label, Layout, Margin, Spinner, TextEdit, ViewportId,
};

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

                            let staged_changes = staged_files();
                            ui.menu_button(
                                format!("{}", if staged_changes > 0 { "*" } else { "" }),
                                |ui| {
                                    let mut close_menu = false;
                                    ui.horizontal(|ui| {
                                        ui.heading("Assets  Settings");
                                        if ui.button("⬇Pull").clicked() {
                                            Action::Pull.enqueue();
                                        }
                                        if working() {
                                            ui.add(Spinner::new());
                                        }
                                        if let Some(Branches {
                                            available,
                                            mut current,
                                        }) = branches()
                                        {
                                            ui.with_layout(
                                                Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    ComboBox::new("branch", "")
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
                                                                    Action::SwitchBranch(branch)
                                                                        .enqueue();
                                                                };
                                                            }
                                                        });
                                                },
                                            );
                                        }
                                    });

                                    ui.label("Repository:");
                                    ui.horizontal(|ui| {
                                        ui.text_edit_singleline(&mut self.new_git_url);
                                        if ui.add(Button::new("Apply").fill(Color32::DARK_RED))
                                            .on_hover_text("This deletes all assets on disk and starts from scratch by cloning the repository!")
                                            .clicked()
                                        {
                                            close_menu = true;

                                            let mut persistant = PersistantState::get();
                                            persistant.git_url = self.new_git_url.clone();
                                            persistant.save();

                                            Action::Nuke.enqueue();
                                        }
                                        ui.add_space(5.0);
                                        if ui
                                            .add(Button::new("Reset"))
                                            .clicked()
                                        {
                                            self.new_git_url = PersistantState::git_url();
                                        }
                                    });

                                    if staged_changes > 0 {
                                        ui.separator();

                                        ui.label(if staged_changes == 1 {
                                            "Commit message for 1 change:".to_owned()
                                        } else {
                                            format!("Commit message for {staged_changes} changes:")
                                        });
                                        ui.horizontal(|ui| {
                                            ui.add(
                                                TextEdit::singleline(&mut self.commit_message)
                                                    .hint_text("Please enter a commit message"),
                                            );
                                            if ui.button("⬆Commit & Push").clicked() {
                                                close_menu = true;
                                                Action::CommitAndPush {
                                                    message: self.commit_message.clone(),
                                                }
                                                .enqueue();
                                                self.commit_message = String::new();
                                            }
                                        });
                                    }

                                    if close_menu {
                                        ui.close_menu();
                                    }
                                },
                            );
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
