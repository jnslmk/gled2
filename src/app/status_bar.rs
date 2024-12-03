use super::{App, GitUiState, PersistantState};
use crate::{
    storage::{
        branches, polynomials_fitting, staged_files, working, Action, Branches, GitCredentials,
        SSH_KEY_PASSPHRASE_ENTRY,
    },
    temperature::temperature,
};
use egui::{
    Button, Color32, ComboBox, Context, Label, Layout, Margin, Spinner, Stroke, TextEdit, Ui,
    ViewportId,
};
use egui_flex::{item, Flex};
use std::fs::read_to_string;

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
                                    ui.label(if n == 1 {
                                        "Fitting one polynomial".to_owned()
                                    } else {
                                        format!("Fitting {n} polynomials")
                                    });
                                    ui.add(Spinner::new());
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
                ui.text_edit_singleline(&mut self.git_ui_state.url);
                Flex::horizontal().show(ui, |flex| {
                    if flex.add(item().grow(1.0), Button::new("Apply").fill(Color32::DARK_RED)).inner
                        .on_hover_text("This deletes all assets on disk and starts from scratch by cloning the repository!")
                        .clicked()
                    {
                        close_menu = true;

                        let mut persistant = PersistantState::get();
                        persistant.git_url = self.git_ui_state.url.clone();
                        if self.git_ui_state.use_agent {
                            persistant.git_credentials = Default::default();
                        }
                        persistant.save();
                        self.git_ui_state.url = GitUiState::default().url;

                        Action::Nuke.enqueue();
                    }

                    if flex.add(item().grow(1.0), Button::new("Reset")).inner
                        .clicked()
                    {
                        self.git_ui_state.url = GitUiState::default().url;
                    }
                });

                if let Some(entry) = SSH_KEY_PASSPHRASE_ENTRY.as_ref() {
                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut self.git_ui_state.use_agent, "Use SSH agent").changed() && !self.git_ui_state.use_agent {
                            if let Some(private_key) = rfd::FileDialog::new()
                                .set_title("Open private ssh key")
                                .pick_file().and_then(|path|read_to_string(path).ok()) {
                                let mut persistant = PersistantState::get();
                                persistant.git_credentials = GitCredentials::Key { private_key };
                                persistant.save();
                            } else {
                                self.git_ui_state.use_agent = true;
                            }
                        }
                        if !self.git_ui_state.use_agent {
                            ui.label("Passphrase:");
                            if ui.add(TextEdit::singleline(&mut self.git_ui_state.passphrase).password(true)).lost_focus()
                            && ui.ctx().input(|input| input.key_pressed(egui::Key::Enter)) {
                                if let Err(err) = entry.set_password(&self.git_ui_state.passphrase) {
                                    log::error!("Could not set passphrase in system keychain: {err}");
                                }
                                self.git_ui_state.passphrase = GitUiState::default().passphrase;
                            }
                        }
                    });
                }
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
                    let commit_and_push = ui
                        .add(
                            TextEdit::singleline(&mut self.git_ui_state.commit_message)
                                .hint_text("Please enter a commit message"),
                        )
                        .lost_focus()
                        && ui.ctx().input(|input| input.key_pressed(egui::Key::Enter));
                    if commit_and_push {
                        Action::CommitAndPush {
                            message: self.git_ui_state.commit_message.clone(),
                        }
                        .enqueue();
                        self.git_ui_state.commit_message.clear();
                    }
                });
        });

        close_menu
    }
}
