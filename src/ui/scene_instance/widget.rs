use crate::storage::asset::scene::grid::GridLocation;
use crate::storage::asset::scene::Scene;
use crate::ui::asset_tree::AssetTree;
use crate::{
    app::timing::Timing, pipeline::group::Groups, storage::asset::scene::instance::SceneInstance,
    ui::gled_slider::GledSlider,
};
use egui::containers::menu::MenuButton;
use egui::{epaint::RectShape, pos2, Align, Button, Color32, CornerRadius, Frame, InnerResponse, Label, Layout, Rect, Response, RichText, Sense, Shape, TextureHandle, Ui, UiBuilder, Vec2, Widget};
use egui_ltreeview::TreeViewState;
use egui_phosphor_icons::icons;
use emath::vec2;
use epaint::{FontFamily, Stroke, StrokeKind};

use crate::storage::asset::project::scene_instance_path::grid_scene_instance_index;
use crate::ui::action::UiAction;

pub(crate) const SCENE_WIDGET_SIZE: f32 = 150.0;
pub struct SceneInstanceWidget<'a> {
    pub scene_instance: &'a mut SceneInstance,
    pub groups: &'a Groups,
    pub svg: Option<TextureHandle>,
    pub size: Vec2,
    pub timing: &'a Timing,
}

const CORNER_RADIUS: u8 = 2;
const INNER_MARGIN: f32 = 6.0;

impl Widget for SceneInstanceWidget<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let name = self.scene_instance.id.to_string();
        let rect = Rect::from_min_size(ui.cursor().min, vec2(SCENE_WIDGET_SIZE, SCENE_WIDGET_SIZE));
        ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
            self.draw_background(ui, &rect);
            let text_response = self.header_bar(name, ui);
            Frame::new().inner_margin(INNER_MARGIN).show(ui, |ui| {
                self.preview(ui);
                self.dimmer(ui);
            });
            text_response
        }) // end outer Frame::show
        .inner
    }
}

impl SceneInstanceWidget<'_> {
    fn header_bar(&mut self, name: String, ui: &mut Ui) -> Response {
        let start_pos = ui.cursor().min;
        let available_width = SCENE_WIDGET_SIZE;

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
                let title_rect = Rect::from_min_size(ui.cursor().min, Vec2::new(available_width - button_width, header_height));
                let resp = scoped_frame(ui, UiBuilder::new().max_rect(title_rect).sense(Sense::drag()), Frame::new().inner_margin(INNER_MARGIN),|ui| {
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

                    if self.scene_instance.active {
                        ui.painter().add(
                            RectShape::filled(
                                text_rec.expand(10.0),
                                5.,
                                Color32::from_black_alpha(50),
                            )
                            .with_blur_width(50.0),
                        );
                    }
                    ui.painter()
                        .add(epaint::TextShape::new(galley_pos, galley, response_color));
                    text_resp
                });

                // Play Button Cell
                let button_rect =
                    Rect::from_min_size(title_rect.right_top(), Vec2::new(button_width, header_height));
                let button_response = scoped_frame(
                    ui,
                    UiBuilder::new().max_rect(button_rect).sense(Sense::click()),
                    Frame::new().fill(Color32::from_gray(30)).corner_radius(
                        CornerRadius { nw: 0, ne: CORNER_RADIUS, sw: 0, se: 0 },
                    ),
                    |ui| {
                        ui.centered_and_justified(|ui| ui.add(Label::new(
                            if self.scene_instance.active {
                                icons::PAUSE.fill().color(Color32::GREEN).size(16.0)
                            } else {
                                icons::PLAY.fill().color(Color32::GREEN).size(16.0)
                            }
                        ).selectable(false)));
                    },
                ).response;
                if button_response.clicked() {
                    self.scene_instance.active = !self.scene_instance.active;
                }
                resp.response
            },
        ).inner
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
        });
    }

    fn dimmer(&self, ui: &mut Ui) {
        ui.scope_builder(UiBuilder::new(), |ui| {
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                Frame::default().show(ui, |ui| {
                    ui.take_available_height();
                    ui.set_width(20.0);

                    let dimmer = self.scene_instance.input_dimmer
                        * self
                            .scene_instance
                            .opacity
                            .value(self.timing.beat_progression());

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
    }

    fn draw_background(&self, ui: &mut Ui, rect: &Rect) {
        ui.painter().rect(
            *rect,
            CORNER_RADIUS,
            Color32::from_gray(100),
            Stroke {
                width: 0.3,
                color: Color32::WHITE,
            },
            StrokeKind::Outside,
        );
        // background glare / shadow
        let glare_shape = if self.scene_instance.active {
            RectShape::filled(rect.expand(10.0), 5., Color32::from_white_alpha(200))
                .with_blur_width(50.)
        } else {
            RectShape::filled(rect.expand(5.0), 5., Color32::from_white_alpha(80))
                .with_blur_width(10.)
        };
        //ui.painter().add(glare_shape);
    }
}

fn scoped_frame<T>(
    ui: &mut Ui,
    ui_builder: UiBuilder,
    frame: Frame,
    add_contents: impl FnOnce(&mut Ui) -> T,
) -> InnerResponse<T> {
    ui.scope_builder(ui_builder, |ui| {
        frame.show(ui, |ui| {
        ui.take_available_space();
        add_contents(ui)
    }).inner
    })
}

pub struct EmptyGridSpot {
    pub location: GridLocation,
}

impl Widget for EmptyGridSpot {
    fn ui(self, ui: &mut Ui) -> Response {
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
                            x: SCENE_WIDGET_SIZE,
                            y: SCENE_WIDGET_SIZE,
                        }),
                    )
                    .ui(ui, |ui| {
                        let scene = AssetTree::<Scene>::show_asset_selection(
                            ui,
                            ui.make_persistent_id(&self.location),
                        );
                        if let Some(scene) = scene {
                            UiAction::AddScene(self.location, scene).enqueue();
                            UiAction::SelectScene(grid_scene_instance_index(self.location))
                                .enqueue();
                            UiAction::InitGPU.enqueue();

                            ui.data_mut(|d| {
                                d.remove::<TreeViewState<usize>>(
                                    ui.make_persistent_id(&self.location),
                                )
                            });
                        }
                    })
                });
            });
        response.response
    }
}
