use super::App;
use crate::{
    storage::{polynomials_fitting, staged_files, working, StorageAction},
    temperature::temperature,
};
use egui::{Button, Context, Label, Layout, Margin, Spinner, TextEdit, Ui, ViewportId};
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

                        #[cfg(not(debug_assertions))]
                        if let Some(version) = crate::update_check::Update::update_available() {
                            if ui.button(format!("Update available: {version}")).clicked() {
                                if let Some(url) = crate::update_check::Update::download_url() {
                                    ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                                }
                            }
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
                self.git_menu(ui);
            },
        );
    }

    fn git_menu(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("Assets ");
            if working() {
                ui.add(Spinner::new());
            }
        });

        let staged_files = staged_files();
        egui::Frame::none()
            .inner_margin(Margin::from(6.0))
            .show(ui, |ui| {
                let mut commit = false;
                ui.add_enabled_ui(staged_files > 0 && !working(), |ui| {
                    ui.label(if staged_files == 1 {
                        "Commit message for 1 change:".to_owned()
                    } else {
                        format!("Commit message for {staged_files} files:")
                    });
                    commit = ui
                        .add(
                            TextEdit::singleline(&mut self.git_commit_message)
                                .hint_text("Please enter a commit message"),
                        )
                        .lost_focus()
                        && ui.ctx().input(|input| input.key_pressed(egui::Key::Enter));
                });

                Flex::horizontal().show(ui, |flex| {
                    if flex.add(item().grow(1.0), Button::new("Commit")).clicked() {
                        commit = true;
                    }

                    if flex.add(item().grow(1.0), Button::new("⬆Push")).clicked() {
                        StorageAction::Push.enqueue();
                    }

                    if flex.add(item().grow(1.0), Button::new("⬇Pull")).clicked() {
                        StorageAction::Pull.enqueue();
                    }
                });

                if commit {
                    StorageAction::Commit {
                        message: self.git_commit_message.clone(),
                    }
                    .enqueue();
                    self.git_commit_message.clear();
                }
            });
    }
}
