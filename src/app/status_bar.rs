use super::{App, PersistantState};
use crate::{
    storage::{branches, polynomials_fitting, staged_files, working, Action, Branches},
    temperature::temperature,
};
use egui::{
    Button, Color32, ComboBox, Context, Label, Layout, Margin, Spinner, Stroke, TextEdit, Ui,
    ViewportId,
};
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
                            self.git_button(ui);
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

    fn git_button(&mut self, ui: &mut Ui) {
        ui.menu_button(
            format!("{}", if staged_files() > 0 { "*" } else { "" }),
            |ui| {
                let close_menu = self.git_menu(ui);
                if close_menu {
                    ui.close_menu();
                }
            },
        );
    }

    fn git_menu(&mut self, ui: &mut Ui) -> bool {
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
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    ComboBox::new("branch", "")
                        .selected_text(&current)
                        .show_ui(ui, |ui| {
                            for branch in available {
                                if ui
                                    .selectable_value(&mut current, branch.clone(), branch.clone())
                                    .changed()
                                {
                                    ui.close_menu();
                                    close_menu = true;
                                    Action::SwitchBranch(branch).enqueue();
                                };
                            }
                        });
                });
            }
        });

        egui::Frame::none()
            .inner_margin(Margin::from(6.0))
            .stroke(Stroke::new(1.0, Color32::DARK_GRAY))
            .show(ui, |ui| {
                ui.label("Repository:");
                ui.text_edit_singleline(&mut self.new_git_url);

                Flex::horizontal().show(ui, |flex| {
                    if flex.add(item().grow(1.0), Button::new("Apply").fill(Color32::DARK_RED)).inner
                        .on_hover_text("This deletes all assets on disk and starts from scratch by cloning the repository!")
                        .clicked()
                    {
                        close_menu = true;

                        let mut persistant = PersistantState::get();
                        persistant.git_url = self.new_git_url.clone();
                        persistant.save();

                        Action::Nuke.enqueue();
                    }

                    if flex.add(item().grow(1.0), Button::new("Reset")).inner
                        .clicked()
                    {
                        let persistant_state = PersistantState::get();
                        self.new_git_url = persistant_state.git_url;
                    }
                });
        });

        let staged_files = staged_files();
        ui.add_enabled_ui(staged_files > 0 && !working(), |ui| {
            egui::Frame::none()
                .inner_margin(Margin::from(6.0))
                .stroke(Stroke::new(1.0, Color32::DARK_GREEN))
                .show(ui, |ui| {
                    ui.label(if staged_files == 1 {
                        "Commit message for 1 change:".to_owned()
                    } else {
                        format!("Commit message for {staged_files} files:")
                    });
                    ui.add(
                        TextEdit::singleline(&mut self.commit_message)
                            .hint_text("Please enter a commit message"),
                    );
                    ui.vertical_centered_justified(|ui| {
                        if ui.button("⬆Commit & Push").clicked() {
                            Action::CommitAndPush {
                                message: self.commit_message.clone(),
                            }
                            .enqueue();
                            self.commit_message = String::new();
                        }
                    });
                });
        });

        close_menu
    }
}
