use crate::storage::asset::scene::grid::GridLocation;
use crate::storage::asset::scene::Scene;
use crate::ui::asset_tree::AssetTree;
use crate::{
    app::timing::Timing,
    pipeline::group::Groups,
    storage::asset::scene::instance::SceneInstance,
    ui::gled_slider::GledSlider,
};
use egui::containers::menu::MenuButton;
use egui::{
    epaint::RectShape, pos2, Align, Button, Color32, CornerRadius, Frame, Layout, Rect, Response,
    RichText, Shadow, Shape, TextureHandle, Ui, UiBuilder, Vec2, Widget,
};
use egui_ltreeview::TreeViewState;
use egui_phosphor_icons::icons;
use epaint::{FontFamily, Stroke};

use crate::storage::asset::project::scene_instance_path::grid_scene_instance_index;
use crate::ui::action::UiAction;

pub(crate) const SCENE_WIDGET_SIZE: f32 = 150.0;
const PREVIEW_SIZE: f32 = 120.0;

pub struct SceneInstanceWidget<'a> {
    pub scene_instance: &'a mut SceneInstance,
    pub groups: &'a Groups,
    pub svg: Option<TextureHandle>,
    pub size: Vec2,
    pub timing: &'a Timing,
}

impl Widget for SceneInstanceWidget<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let active = self.scene_instance.active;
        let preview_color = self.scene_instance.color;
        let name = self.scene_instance.id.to_string();

        let beat_progression = self.timing.beat_progression();

        Frame::default()
            .fill(Color32::from(preview_color))
            .stroke(Stroke {
                width: 0.3,
                color: Color32::WHITE,
            })
            .shadow(if active {
                Shadow {
                    offset: [0, 0],
                    blur: 6,
                    spread: 2,
                    color: Color32::from_white_alpha(150),
                }
            } else {
                Shadow::NONE
            })
            .corner_radius(2)
            .inner_margin(14)
            .show(ui, |ui| {
                // ensure all available space is used
                ui.set_width(SCENE_WIDGET_SIZE);
                ui.set_height(SCENE_WIDGET_SIZE);

                // background glare / shadow
                let glare_rect = ui.cursor();
                let glare_shape = if active {
                    RectShape::filled(glare_rect.expand(10.0), 5., Color32::from_white_alpha(200))
                        .with_blur_width(50.)
                } else {
                    RectShape::filled(glare_rect.expand(20.0), 5., Color32::from_black_alpha(100))
                        .with_blur_width(10.)
                };
                ui.painter().add(glare_shape);

                let start_pos = ui.cursor().min;
                let available_width = ui.available_width();

                // --- Header Strip (Name + Play Button) ---
                // Height: 20.0
                let header_height = 20.0;
                let header_rect =
                    Rect::from_min_size(start_pos, Vec2::new(available_width, header_height));

                let text_response = ui.scope_builder(UiBuilder::new().max_rect(header_rect), |ui| {
                    let name_width = header_rect.width() * 0.6;
                    let button_width = header_rect.width() - name_width;

                    // Name Cell
                    let name_rect =
                        Rect::from_min_size(header_rect.min, Vec2::new(name_width, header_height));

                    let text_resp = ui.scope_builder(UiBuilder::new().max_rect(name_rect), |ui| {
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            // drop shadow behind the text for better readability
                            let title_label = egui::Label::new(
                                RichText::new(name)
                                    .family(FontFamily::Name("Bold".into()))
                                    .size(12.0)
                                    .color(Color32::WHITE),
                            )
                                .truncate();

                            // when accessing text layout information,
                            // painting has to be done manually instead
                            let (galley_pos, galley, text_resp) = title_label.layout_in_ui(ui);
                            let response_color = ui.style().visuals.text_color();
                            let text_rec = galley.rect.translate(galley_pos.to_vec2());

                            if active {
                                ui.painter().add(
                                    RectShape::filled(
                                        text_rec.expand(10.0),
                                        5.,
                                        Color32::from_black_alpha(50),
                                    )
                                        .with_blur_width(50.0),
                                );
                            }
                            ui.painter().add(epaint::TextShape::new(
                                galley_pos,
                                galley,
                                response_color,
                            ));
                            text_resp
                        })
                    });

                    // Play Button Cell
                    let button_rect = Rect::from_min_size(
                        header_rect.min + Vec2::new(name_width, 0.0),
                        Vec2::new(button_width, header_height),
                    );

                    ui.scope_builder(UiBuilder::new().max_rect(button_rect), |ui| {
                        let button_response = ui.add_sized(
                            ui.available_size(),
                            Button::new(if active {
                                icons::PAUSE.fill().color(Color32::GREEN).size(16.0)
                            } else {
                                icons::PLAY.fill().color(Color32::GREEN).size(16.0)
                            })
                                .stroke(Stroke::new(0.3, Color32::WHITE))
                        );
                        // this is a workaround for https://github.com/emilk/egui/issues/7767
                        if button_response.drag_started() || button_response.clicked() {
                            self.scene_instance.active = !self.scene_instance.active;
                        }
                    });
                    text_resp.inner
                }).inner;

                // --- Preview Strip (Preview + Dimmer) ---
                // Gap: 5.0
                let gap = 5.0;
                let body_y = start_pos.y + header_height + gap;

                // Preview Box
                let preview_rect = Rect::from_min_size(
                    pos2(start_pos.x, body_y),
                    Vec2::new(PREVIEW_SIZE, PREVIEW_SIZE),
                );

                ui.scope_builder(UiBuilder::new().max_rect(preview_rect), |ui| {
                    Frame::default().fill(Color32::BLACK).show(ui, |ui| {
                        ui.set_height(PREVIEW_SIZE);
                        ui.set_width(PREVIEW_SIZE);
                        for texture_id in self.scene_instance.texture_ids() {
                            ui.painter().add(Shape::Rect(
                                RectShape::filled(
                                    ui.cursor(),
                                    CornerRadius::default(),
                                    Color32::WHITE,
                                )
                                    .with_texture(
                                        texture_id,
                                        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                                    ),
                            ));
                        }
                    });
                });

                // Dimmer Slider
                let dimmer_x = start_pos.x + PREVIEW_SIZE;
                // Use remainder of width
                let dimmer_width = available_width - PREVIEW_SIZE;
                let dimmer_rect = Rect::from_min_size(
                    pos2(dimmer_x, body_y),
                    Vec2::new(dimmer_width, PREVIEW_SIZE),
                );

                ui.scope_builder(UiBuilder::new().max_rect(dimmer_rect), |ui| {
                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        egui::Frame::default().show(ui, |ui| {
                            ui.take_available_height();
                            ui.set_width(20.0);

                            let dimmer = self.scene_instance.input_dimmer
                                * self.scene_instance.opacity.value(beat_progression);

                            ui.add(GledSlider {
                                real_value: if self.scene_instance.active {
                                    dimmer
                                } else {
                                    0.0
                                },
                                size: 20.0,
                                max_value: 100.0,
                                value: &mut self.scene_instance.opacity.multiplier(),
                                show_label: false,
                                horizontal: false,
                            });
                        });
                    });
                });
                text_response.inner
            }) // end outer Frame::show
            .inner
    }
}

pub struct EmptyGridSpot {
    pub location: GridLocation,
}

impl Widget for EmptyGridSpot {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let response = Frame::default()
            .stroke(Stroke {
                width: 0.3,
                color: Color32::WHITE,
            })
            .corner_radius(2)
            .show(ui, |ui| {
                ui.with_layout(Layout::top_down(Align::Center), |ui| {
                    MenuButton::from_button(
                        Button::new(
                            icons::PLUS
                                .regular()
                                .size(60.0)
                                .color(Color32::from_gray(120)),
                        )
                            .fill(Color32::TRANSPARENT)
                            .min_size(Vec2 {
                                x: SCENE_WIDGET_SIZE + 28.,
                                y: SCENE_WIDGET_SIZE + 28.,
                            }),
                    )
                        .ui(
                            ui,
                            |ui| {
                                let scene = AssetTree::<Scene>::show_asset_selection(
                                    ui,
                                    ui.make_persistent_id(&self.location),
                                );
                                if let Some(scene) = scene {
                                    UiAction::AddScene(self.location, scene).enqueue();
                                    UiAction::SelectScene(grid_scene_instance_index(self.location)).enqueue();
                                    UiAction::InitGPU.enqueue();

                                    ui.data_mut(|d| {
                                        d.remove::<TreeViewState<usize>>(ui.make_persistent_id(&self.location))
                                    });
                                }
                            },
                        )
                });
            });
        response.response
    }
}