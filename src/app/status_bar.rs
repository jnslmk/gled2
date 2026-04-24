use super::App;
use crate::{
    app::timing::LINK_ACTIVE_COLOR,
    storage::{
        action::StorageAction, asset::{curve::polynomial::polynomials_fitting, project::{GridHighlight}}, staged_files, working,
    },
    ui::temperature::temperature,
};
use egui::{
    Button, DragValue, FontSelection, Label, Layout, Margin, RichText, Slider, Spinner, TextEdit,
    Ui, UiKind, ViewportId,
    containers::menu::{MenuButton, MenuConfig},
};
use egui_flex::{Flex, item};
use egui_phosphor_icons::icons;
use emath::Align;
use epaint::text::{LayoutJob, TextFormat};

impl App {
    pub fn status_bar(&mut self, ui: &mut Ui, viewport_id: Option<ViewportId>) {
        egui::Panel::bottom(format!("{viewport_id:?} status bar")).show_inside(ui, |ui| {
            egui::Frame::NONE
                .inner_margin(Margin::from(1.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(Label::new(temperature(ui)).selectable(false));

                        if let Some(framerate) = self.timing.framerate() {
                            ui.add_space(8.0);
                            let mut layout_job = LayoutJob::default();
                            icons::GAUGE.regular().append_to(
                                &mut layout_job,
                                ui.style(),
                                egui::FontSelection::Default,
                                egui::Align::Center,
                            );
                            layout_job.append(
                                format!(" {framerate:.0} fps",).as_str(),
                                0.0,
                                TextFormat::default(),
                            );
                            ui.add(Label::new(layout_job).selectable(false));
                        }

                        let (incoming, outgoing) = self.network_stats;
                        ui.add_space(8.0);
                        let mut layout_job = LayoutJob::default();
                        icons::NETWORK.regular().append_to(
                            &mut layout_job,
                            ui.style(),
                            FontSelection::Default,
                            Align::Center,
                        );
                        layout_job.append("  ", 0.0, TextFormat::default());
                        icons::CARET_DOWN.regular().append_to(
                            &mut layout_job,
                            ui.style(),
                            FontSelection::Default,
                            Align::Center,
                        );
                        layout_job.append(
                            format!(" {:.2} MBit/s  ", incoming).as_str(),
                            0.0,
                            TextFormat::default(),
                        );
                        icons::CARET_UP.regular().append_to(
                            &mut layout_job,
                            ui.style(),
                            FontSelection::Default,
                            Align::Center,
                        );
                        layout_job.append(
                            format!(" {:.2} MBit/s", outgoing).as_str(),
                            0.0,
                            TextFormat::default(),
                        );
                        ui.add(Label::new(layout_job).selectable(false));

                        let connected_ableton_peers = crate::app::timing::CONNECTED_PEERS
                            .load(std::sync::atomic::Ordering::Relaxed);
                        let show_link_read_only_toggle =
                            connected_ableton_peers > 0 || self.timing.ableton_link_read_only();

                        if show_link_read_only_toggle {
                            ui.add_space(8.0);
                            if connected_ableton_peers > 0 {
                                let mut layout_job = LayoutJob::default();
                                icons::METRONOME
                                    .regular()
                                    .color(LINK_ACTIVE_COLOR)
                                    .append_to(
                                        &mut layout_job,
                                        ui.style(),
                                        egui::FontSelection::Default,
                                        egui::Align::Center,
                                    );
                                RichText::new(format!(
                                    " {connected_ableton_peers} Ableton Link peer{} ",
                                    if connected_ableton_peers == 1 {
                                        ""
                                    } else {
                                        "s"
                                    }
                                ))
                                .color(LINK_ACTIVE_COLOR)
                                .append_to(
                                    &mut layout_job,
                                    ui.style(),
                                    egui::FontSelection::Default,
                                    egui::Align::Center,
                                );

                                ui.add(Label::new(layout_job).selectable(false));
                            }

                            let mut read_only = self.timing.ableton_link_read_only();
                            let changed = ui
                                .scope(|ui| {
                                    let visuals = &mut ui.style_mut().visuals;
                                    visuals.widgets.inactive.bg_stroke.color = LINK_ACTIVE_COLOR;
                                    visuals.widgets.hovered.bg_stroke.color = LINK_ACTIVE_COLOR;
                                    visuals.widgets.active.bg_stroke.color = LINK_ACTIVE_COLOR;
                                    visuals.widgets.inactive.fg_stroke.color = LINK_ACTIVE_COLOR;
                                    visuals.widgets.hovered.fg_stroke.color = LINK_ACTIVE_COLOR;
                                    visuals.widgets.active.fg_stroke.color = LINK_ACTIVE_COLOR;

                                    ui.checkbox(
                                        &mut read_only,
                                        RichText::new("Read-only").color(LINK_ACTIVE_COLOR),
                                    )
                                    .on_hover_text(
                                        "Ignore local tempo changes (tap, x1/2, x2, slider, OSC, MIDI).",
                                    )
                                    .changed()
                                })
                                .inner;

                            if changed {
                                self.timing.set_ableton_link_read_only(read_only);
                            }
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
                            self.grid_button(ui);
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

    fn grid_button(&mut self, ui: &mut Ui) {
        if let Some(project) = self.project.as_mut() {
            let mut layout_job = LayoutJob::default();
            icons::SQUARES_FOUR.fill().append_to(
                &mut layout_job,
                ui.style(),
                egui::FontSelection::Default,
                egui::Align::Center,
            );
            layout_job.append(" Grid", 0.0, TextFormat::default());

            ui.menu_button(layout_job, |ui| {
                ui.label("Grid Size");

                let mut grid_width = project.grid_width();
                let mut grid_height = project.grid_height();

                ui.horizontal(|ui| {
                    ui.label("Width");
                    ui.add(DragValue::new(&mut grid_width).range(2..=64));
                    ui.label("Height");
                    ui.add(DragValue::new(&mut grid_height).range(2..=64));
                });

                if grid_width != project.grid_width() || grid_height != project.grid_height() {
                    project.set_grid_size(grid_width, grid_height);

                    let max_col = project.grid_width() - 1;
                    let max_row = project.grid_height() - 1;
                    self.selected_scene_instance.col =
                        self.selected_scene_instance.col.min(max_col);
                    self.selected_scene_instance.row =
                        self.selected_scene_instance.row.min(max_row);
                }

                ui.separator();
                ui.label("Highlight");
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut project.grid_highlight,
                        GridHighlight::Row,
                        "Last Row",
                    );
                    ui.selectable_value(
                        &mut project.grid_highlight,
                        GridHighlight::Column,
                        "Last Column",
                    );
                    ui.selectable_value(&mut project.grid_highlight, GridHighlight::None, "None");
                });

                ui.separator();

                ui.spacing_mut().slider_width = 195.0;

                ui.label("Scene preview size");
                    if ui
                        .add(
                            Slider::new(self.persistant_state.effects_size_mut(), 50.0..=500.0)
                                .show_value(false),
                        )
                        .changed()
                    {
                        self.persistant_state.save();
                    }
            });
            ui.add_space(4.0);
        }
    }

    fn git_button(&mut self, ui: &mut Ui) {
        let mut layout_job = LayoutJob::default();
        icons::GIT_BRANCH.regular().append_to(
            &mut layout_job,
            ui.style(),
            egui::FontSelection::Default,
            egui::Align::Center,
        );
        layout_job.append(
            format!(" git{}", if staged_files() > 0 { "*" } else { "" }).as_str(),
            0.0,
            TextFormat::default(),
        );

        MenuButton::from_button(Button::new(layout_job))
            .config(
                MenuConfig::new().close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside),
            )
            .ui(ui, |ui| {
                self.git_menu(ui);
            });
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
                let mut close_menu = false;
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
                        close_menu = true;
                    }

                    if flex.add(item().grow(1.0), Button::new("⬆Push")).clicked() {
                        StorageAction::Push.enqueue();
                        close_menu = true;
                    }

                    if flex.add(item().grow(1.0), Button::new("⬇Pull")).clicked() {
                        StorageAction::Pull.enqueue();
                        close_menu = true;
                    }
                });

                if commit {
                    StorageAction::Commit {
                        message: self.git_commit_message.clone(),
                    }
                    .enqueue();
                    self.git_commit_message.clear();
                    close_menu = true;
                }

                if close_menu {
                    ui.close_kind(UiKind::Menu);
                }
            });
    }
}
