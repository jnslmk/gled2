use super::App;
use crate::ui::scoped_frame;
use crate::{
    pipeline::preview::Preview,
    ui::{
        asset_tree::{TreeId, TreeSelection},
        gled_slider::GledSlider,
    },
};
use egui::{load::SizedTexture, Button, Color32, Id, Image, Layout, Ui, UiBuilder, Vec2};
use egui::Frame;
use egui_ltreeview::TreeViewState;
use egui_phosphor_icons::icons;
use emath::{vec2, Align};
use epaint::text::{LayoutJob, TextFormat};
use epaint::Stroke;
use std::time::{SystemTime, UNIX_EPOCH};

impl App {
    pub fn palette_asset_tree_id(&mut self, ui: &mut Ui) -> Id {
        *self
            .palette_asset_tree_id
            .get_or_insert_with(|| ui.make_persistent_id("preview palette tree"))
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn preview(&mut self, ui: &mut egui::Ui) {
        let palette_asset_tree_id = self.palette_asset_tree_id(ui);
        let palette_tree_id = self
            .project
            .as_ref()
            .and_then(|project| project.palette)
            .map(TreeId::File);

        if let Some(palette_tree_id) = palette_tree_id.as_ref() {
            let mut state = TreeViewState::load(ui, palette_asset_tree_id).unwrap_or_default();
            if state.selected().iter().next().is_none()
                && let Some(index) = self
                    .palette_asset_tree
                    .find_index(palette_tree_id, &mut self.collections)
            {
                state.set_selected(vec![index]);
                state.store(ui, palette_asset_tree_id);
            }
        }

        let height = ui.available_height();
        ui.horizontal(|mut ui| {
            let size = Vec2::splat(height.min(ui.available_width()));

            let svg_texture = self.svg_texture(ui.ctx());
            let res = svg_texture.map(|svg_texture_handle| {
                ui.add(
                    Image::new(SizedTexture::new(svg_texture_handle.id(), size))
                        .bg_fill(Color32::BLACK),
                )
            });
            let preview = Image::new(SizedTexture::new(Preview::texture_id(), size));
            match res {
                Some(res) => {
                    ui.put(res.rect, preview);
                }
                None => {
                    ui.add(preview);
                }
            }

            if let Some(project) = &mut self.project {
                ui.add_space(ui.available_width() - (300.0 + 16.0 + 40.0 + 16.0));
                ui.scope(|ui| {
                    ui.set_max_width(300.0);
                    if self.palette_asset_tree.show(
                        ui,
                        palette_asset_tree_id,
                        &mut self.collections,
                    ) {
                        match self.palette_asset_tree.selected() {
                            TreeSelection::Asset(palette) => {
                                project.palette = Some(palette.id);
                            }
                            _ => {
                                let mut state = TreeViewState::load(ui, palette_asset_tree_id)
                                    .unwrap_or_default();
                                if let Some(index) =
                                    palette_tree_id.as_ref().and_then(|palette_tree_id| {
                                        self.palette_asset_tree
                                            .find_index(palette_tree_id, &mut self.collections)
                                    })
                                {
                                    state.set_selected(vec![index]);
                                    state.store(ui, palette_asset_tree_id);
                                }
                            }
                        };
                    }
                });

                let mut rect = ui.available_rect_before_wrap();
                *rect.top_mut() += 16.0;
                *rect.left_mut() = rect.right() - 56.0;
                *rect.bottom_mut() -= 16.0;

                scoped_frame(
                    ui,
                    UiBuilder::new().max_rect(rect).layout(Layout::top_down_justified(Align::Min)),
                    Frame::default(),
                    |ui| {
                        let (top, mut bottom) = rect.split_top_bottom_at_y(rect.height() + 20.0);

                        let preview_value = if self.blackout {0.0} else {project.main_dimmer};
                        ui.put(top, GledSlider::new(&mut project.main_dimmer, 100.0)
                            .size(40.0)
                            .show_label()
                            .preview_value(preview_value));

                        let underlined = TextFormat {
                            underline: Stroke::new(1.0, Color32::GRAY),
                            ..Default::default()
                        };
                        let mut blackout_text = LayoutJob::default();
                        blackout_text.append("B", 0.0, underlined);
                        blackout_text.append("lackout", 0.0, TextFormat::default());

                        let mut blackout = Button::new(icons::LIGHTBULB);
                        if self.blackout
                            && SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .map(|d| d.as_millis() / 200 % 2 == 0)
                            .unwrap_or_default()
                        {
                            blackout = blackout.fill(Color32::DARK_RED);
                        }
                        bottom.set_width(40.0);
                        if ui.add_sized(vec2(10.0, 10.0), blackout).clicked()
                            || project.blackout_input_is_new()
                        {
                            self.blackout = !self.blackout;
                        }

                        self.blackout_hold = project.blackout_hold_input_is_live();
                    });
            }
        });
    }
}