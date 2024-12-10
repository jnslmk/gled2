use crate::{app::{GitUiState, PersistantState}, storage::{branches, Action, Branches}, viewport_builder::default_viewport_builder};
use egui::{Button, Color32, ComboBox, Context, Id, Layout, RichText, TextEdit, Vec2, ViewportId};
use egui_flex::{item, Flex};
use home::home_dir;

#[derive(Default)]
pub struct GitConfigWindow {
    open: bool,
    git_ui_state: GitUiState,
}

impl GitConfigWindow {
    pub fn update(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }
        
        ctx.show_viewport_immediate(
            ViewportId(Id::new("git config window")),
            default_viewport_builder()
                .with_title("Gled: Shortcuts")
                .with_inner_size(Vec2::new(500.0, 180.0))
                .with_min_inner_size(Vec2::new(500.0, 180.0))
                .with_resizable(false)
                .with_max_inner_size(Vec2::new(500.0, 180.0)),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.close();
                    }
                });

                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.with_layout(Layout::top_down_justified(egui::Align::Center), |ui| {
                        ui.label(RichText::new("Any change here can nuke all you progress!").color(Color32::RED).heading());
                    });
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {ui.label("Repository:");
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label("Branch:");
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(&mut self.git_ui_state.url);
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
                                                Action::SwitchBranch(branch).enqueue();
                                            };
                                        }
                                    });
                            });
                        }
                    });

                    Flex::horizontal().show(ui, |flex| {
                        if flex.add(item().grow(1.0), Button::new("Apply").fill(Color32::DARK_RED)).inner
                            .on_hover_text("This deletes all assets on disk and starts from scratch by cloning the repository!")
                            .clicked()
                        {
                            let mut persistant = PersistantState::get();
                            persistant.git_url = self.git_ui_state.url.clone();
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

                    ui.add_space(30.0);

                    ui.horizontal(|ui| {
                        ui.vertical_centered_justified(|ui| {
                            let choose_private_key_text = match PersistantState::git_credentials().private_key_path() {
                                Some(path) => format!("Choose private key (current: {})", path.display()),
                                None => "Choose private key".to_owned(),
                            };

                            if ui.button(choose_private_key_text).clicked() {
                                std::thread::spawn(|| {
                                    let mut file_dialog = rfd::FileDialog::new().set_title("Choose private key");
                                    if let Some(home) = home_dir() {
                                        file_dialog = file_dialog.set_directory(home.join(".ssh"))
                                    }
                                    if let Some(private_key_path) = 
                                        file_dialog.pick_file() {
                                        let mut persistant = PersistantState::get();
                                        persistant.git_credentials.set_private_key_path(private_key_path);
                                        persistant.save();
                                    }
                                });
                            }
                        });
                    });

                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut self.git_ui_state.use_passphrase, "Use passphrase").changed() {
                            let mut persistant = PersistantState::get();
                            persistant.git_credentials.set_use_passphrase(self.git_ui_state.use_passphrase);
                            persistant.save();
                        }

                        if self.git_ui_state.use_passphrase {
                            ui.label("Passphrase:");
                            if ui.add(TextEdit::singleline(&mut self.git_ui_state.passphrase).hint_text("Please enter key password").password(true)).lost_focus()
                            && ui.ctx().input(|input| input.key_pressed(egui::Key::Enter)) {
                                let mut persistant = PersistantState::get();
                                persistant.git_credentials.set_passphrase(self.git_ui_state.passphrase.clone());
                                persistant.save();
                                self.git_ui_state.passphrase = GitUiState::default().passphrase;
                            }
                        }
                    });                   
                });
            },
        );
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }
}
