use crate::audio::sound_data::SoundData;
use crate::storage::asset::project::scene_instance_path::grid_scene_instance_index;
use crate::storage::asset::scene::Scene;
use crate::storage::asset::scene::grid::GridLocation;
use crate::storage::collections::Collections;
use crate::ui::action::UiAction;
use crate::ui::asset_tree::AssetTree;
use crate::{
    app::timing::Timing, pipeline::group::Groups, storage::asset::scene::instance::SceneInstance,
    ui::gled_slider::GledSlider,
};
use egui::containers::menu::MenuButton;
use egui::{
    Align, Button, Color32, CornerRadius, Frame, InnerResponse, Label, Layout, Rect, Response,
    RichText, Sense, Shape, TextureHandle, Ui, UiBuilder, Vec2, Widget, epaint::RectShape, pos2,
};
use egui_ltreeview::TreeViewState;
use egui_phosphor_icons::icons;
use emath::vec2;
use epaint::{FontFamily, Stroke, StrokeKind};

pub struct SceneInstanceWidget<'a> {
    pub scene_instance: &'a mut SceneInstance,
    pub groups: &'a Groups,
    pub svg: Option<TextureHandle>,
    pub size: Vec2,
    pub timing: &'a Timing,
    pub collections: &'a Collections,
    pub effects_size: f32,
    pub sound_data: &'a SoundData,
}

const CORNER_RADIUS: u8 = 2;
const INNER_MARGIN: f32 = 6.0;

impl Widget for SceneInstanceWidget<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let name = self.scene_instance.name.clone();
        let scene_visible = self.scene_instance.active || self.scene_instance.flash;
        let rect = Rect::from_min_size(ui.cursor().min, Vec2::splat(self.effects_size));
        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
            self.draw_background(ui, &rect);
            let (text_response, button_rect) = self.header_bar(name, ui);
            Frame::new().inner_margin(INNER_MARGIN).show(ui, |ui| {
                self.preview(ui);
                self.dimmer(ui, self.collections, self.sound_data);
            });
            if !scene_visible {
                ui.painter()
                    .rect_filled(rect.expand(1.), 0., Color32::from_black_alpha(200));
            }
            self.draw_enable_button(ui, button_rect);
            text_response
        }) // end outer Frame::show
        .inner
    }
}

impl SceneInstanceWidget<'_> {
    fn header_bar(&mut self, name: String, ui: &mut Ui) -> (Response, Rect) {
        let start_pos = ui.cursor().min;
        let available_width = self.effects_size;

        // --- Header Strip (Name + Play Button) ---
        let header_height = 30.;
        let header_rect = Rect::from_min_size(start_pos, Vec2::new(available_width, header_height));
        let button_width = 50.;

        scoped_frame(
            ui,
            UiBuilder::new().max_rect(header_rect),
            Frame::new()
                .fill(Color32::from(self.scene_instance.color))
                .corner_radius(CornerRadius {
                    nw: CORNER_RADIUS,
                    ne: CORNER_RADIUS,
                    sw: 0,
                    se: 0,
                }),
            |ui| {
                // Name Cell
                let title_rect = Rect::from_min_size(
                    ui.cursor().min,
                    Vec2::new(available_width - button_width, header_height),
                );
                let resp = scoped_frame(
                    ui,
                    UiBuilder::new()
                        .max_rect(title_rect)
                        .sense(Sense::click_and_drag()),
                    Frame::new().inner_margin(INNER_MARGIN),
                    |ui| {
                        ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                            // drop shadow behind the text for better readability
                            let title_label = Label::new(
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

                            ui.painter().add(
                                RectShape::filled(text_rec, 5., Color32::from_black_alpha(60))
                                    .with_blur_width(20.0),
                            );
                            ui.painter().add(epaint::TextShape::new(
                                galley_pos,
                                galley,
                                response_color,
                            ));
                            text_resp
                        });
                    },
                );

                // Play Button Cell
                let button_rect = Rect::from_min_size(
                    title_rect.right_top(),
                    Vec2::new(button_width, header_height),
                )
                .shrink(2.);
                self.draw_enable_button(ui, button_rect);
                ui.painter().line(
                    vec![header_rect.left_bottom(), header_rect.right_bottom()],
                    Stroke::new(0.3, Color32::WHITE),
                );
                (resp.response, button_rect)
            },
        )
        .inner
    }

    fn draw_enable_button(&mut self, ui: &mut Ui, button_rect: Rect) {
        let button_response = ui.put(
            button_rect,
            Button::new(if self.scene_instance.active {
                icons::PAUSE.fill().color(Color32::GREEN).size(18.0)
            } else {
                icons::PLAY
                    .fill()
                    .color(Color32::GREEN.blend(Color32::from_black_alpha(100)))
                    .size(18.0)
            })
            .corner_radius(CornerRadius {
                nw: 0,
                ne: CORNER_RADIUS,
                sw: 0,
                se: 0,
            })
            .fill(if self.scene_instance.active {
                Color32::from_black_alpha(50)
            } else {
                Color32::from(self.scene_instance.color).blend(Color32::from_black_alpha(160))
            }),
        );
        if button_response.clicked() {
            self.scene_instance.active = !self.scene_instance.active;
        }
    }

    fn preview(&self, ui: &mut Ui) {
        ui.scope_builder(UiBuilder::new(), |ui| {
            let height = ui.available_height();
            let preview_rect = Rect::from_min_size(ui.cursor().min, vec2(height, height));
            ui.painter().rect_filled(preview_rect, 0., Color32::BLACK);
            for texture_id in self.scene_instance.texture_ids() {
                ui.painter().add(Shape::Rect(
                    RectShape::filled(preview_rect, CornerRadius::default(), Color32::WHITE)
                        .with_texture(
                            texture_id,
                            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                        ),
                ));
            }
            if !self.scene_instance.active && !self.scene_instance.flash {
                ui.painter().rect_filled(
                    preview_rect.expand(1.),
                    0.,
                    Color32::from_black_alpha(100),
                );
            }
        });
    }

    fn dimmer(&mut self, ui: &mut Ui, collections: &Collections, sound_data: &SoundData) {
        ui.scope_builder(UiBuilder::new(), |ui| {
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                Frame::default().show(ui, |ui| {
                    ui.take_available_height();
                    ui.set_width(20.0);

                    let dimmer = self.scene_instance.input_dimmer
                        * self.scene_instance.opacity.value(
                            self.timing.beat_progression(),
                            collections,
                            sound_data,
                        );

                    ui.add(
                        GledSlider::new(&mut self.scene_instance.opacity.multiplier, 100.0)
                            .preview_value(
                                if self.scene_instance.active || self.scene_instance.flash {
                                    dimmer
                                } else {
                                    0.0
                                },
                            )
                            .size(20.0),
                    );
                });
            });
        });
    }

    fn draw_background(&self, ui: &mut Ui, rect: &Rect) {
        ui.painter().rect(
            *rect,
            CORNER_RADIUS,
            Color32::from_gray(20),
            Stroke {
                width: 0.3,
                color: Color32::WHITE,
            },
            StrokeKind::Outside,
        );
        // background glare / shadow
        let glare_shape = RectShape::filled(
            rect.expand(2.),
            CORNER_RADIUS,
            Color32::from_white_alpha(80),
        )
        .with_blur_width(10.);
        let mut painter = ui.ctx().layer_painter(ui.layer_id());
        painter.set_clip_rect(rect.intersect(ui.clip_rect()));
        painter.add(glare_shape);
    }
}

fn scoped_frame<T>(
    ui: &mut Ui,
    ui_builder: UiBuilder,
    frame: Frame,
    add_contents: impl FnOnce(&mut Ui) -> T,
) -> InnerResponse<T> {
    ui.scope_builder(ui_builder, |ui| {
        frame
            .show(ui, |ui| {
                ui.take_available_space();
                add_contents(ui)
            })
            .inner
    })
}

pub struct EmptyGridSpot<'a> {
    pub location: GridLocation,
    pub collections: &'a mut Collections,
    pub effects_size: f32,
}

impl<'a> Widget for EmptyGridSpot<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let rect = Rect::from_min_size(ui.cursor().min, Vec2::splat(self.effects_size));
        const EMPTY_COLOR: Color32 = Color32::from_gray(80);
        let response = scoped_frame(
            ui,
            UiBuilder::new().max_rect(rect),
            Frame::default()
                .stroke(Stroke {
                    width: 0.3,
                    color: EMPTY_COLOR,
                })
                .corner_radius(2),
            |ui| {
                ui.with_layout(Layout::top_down(Align::Center), |ui| {
                    MenuButton::from_button(
                        Button::new(icons::PLUS.regular().size(60.0).color(EMPTY_COLOR))
                            .fill(Color32::TRANSPARENT)
                            .min_size(Vec2::splat(self.effects_size)),
                    )
                    .ui(ui, |ui| {
                        let scene = AssetTree::<Scene>::show_asset_selection(
                            ui,
                            ui.make_persistent_id(self.location),
                            self.collections,
                        );
                        if let Some(scene) = scene {
                            UiAction::AddScene(self.location, scene).enqueue();
                            UiAction::SelectScene(grid_scene_instance_index(self.location))
                                .enqueue();

                            ui.data_mut(|d| {
                                d.remove::<TreeViewState<usize>>(
                                    ui.make_persistent_id(self.location),
                                )
                            });
                        }
                    })
                });
            },
        );
        response.response
    }
}
