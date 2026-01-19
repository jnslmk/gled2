use super::App;
use crate::{
    app::timing::LINK_ACTIVE_COLOR,
    network_stats,
    storage::{
        action::StorageAction, asset::curve::polynomial::polynomials_fitting, staged_files, working,
    },
    ui::temperature::temperature,
};
use egui::{Button, Label, Layout, Margin, RichText, Spinner, TextEdit, Ui, ViewportId};
use egui_flex::{Flex, item};

impl App {
    pub fn status_bar(&mut self, ui: &mut Ui, viewport_id: Option<ViewportId>) {
        egui::TopBottomPanel::bottom(format!("{viewport_id:?} status bar")).show_inside(ui, |ui| {
            egui::Frame::NONE
                .inner_margin(Margin::from(1.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(Label::new(temperature()).selectable(false));

                        if let Some(framerate) = self.timing.framerate() {
                            ui.add_space(8.0);
                            ui.add(
                                Label::new(format!(
                                    "{} {framerate:.0} fps",
                                    egui_phosphor::regular::GAUGE
                                ))
                                .selectable(false),
                            );
                        }

                        let network_stats = network_stats::stats();
                        if !network_stats.is_empty() {
                            ui.add_space(8.0);
                            ui.add(Label::new(network_stats).selectable(false));
                        }

                        let connected_ableton_peers = crate::app::timing::CONNECTED_PEERS
                            .load(std::sync::atomic::Ordering::Relaxed);
                        if connected_ableton_peers > 0 {
                            ui.add_space(8.0);
                            ui.add(
                                Label::new(
                                    RichText::new(format!(
                                        " {} {connected_ableton_peers} Ableton Link peer{} ",
                                        egui_phosphor::regular::METRONOME,
                                        if connected_ableton_peers == 1 {
                                            ""
                                        } else {
                                            "s"
                                        }
                                    ))
                                    .color(LINK_ACTIVE_COLOR),
                                )
                                .selectable(false),
                            );
                        }

                        #[cfg(not(debug_assertions))]
                        if let Some(version) = crate::ui::update_check::Update::update_available() {
                            if ui.button(format!("Update available: {version}")).clicked() {
                                if let Some(url) = crate::ui::update_check::Update::download_url() {
                                    ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                                }
                            }
                        }

                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(4.0);
                            self.git_button(ui);
                            ui.add_space(4.0);
                            match polynomials_fitting() {
                                0 => (),
                                n => {
                                    ui.add(
                                        Label::new(if n == 1 {
                                            "Fitting one polynomial".to_owned()
                                        } else {
                                            format!("Fitting {n} polynomials")
                                        })
                                        .selectable(false),
                                    );
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
            format!(
                "{} git{}",
                egui_phosphor::regular::GIT_BRANCH,
                if staged_files() > 0 { "*" } else { "" }
            ),
            |ui| {
                self.git_menu(ui);
            },
        );
    }

    fn git_menu(&mut self, ui: &mut Ui) {
        ui.set_max_width(300.0);
        ui.horizontal(|ui| {
            ui.heading("Assets ");
            if working() {
                ui.add(Spinner::new());
            }
        });

        let staged_files = staged_files();
        egui::Frame::NONE
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
