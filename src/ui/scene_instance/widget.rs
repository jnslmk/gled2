use crate::storage::asset::project::Project;
use crate::storage::asset::scene::Scene;
use crate::storage::asset_id::AssetId;
use crate::ui::ChangeButton;
use crate::{
    app::timing::Timing,
    pipeline::{constants::PREVIEW_TEXTURE_SIZE, group::Groups},
    storage::asset::{
        project::{DeckPath, scene_instance_path::SceneInstancePathId},
        scene::instance::SceneInstance,
    },
    ui::gled_slider::GledSlider,
};
use egui::{
    Align, Button, Color32, ColorImage, Context, CornerRadius, Frame, Layout, Rect, Response,
    RichText, Sense, Shadow, Shape, TextureHandle, TextureId, TextureOptions, Ui, Vec2, Widget,
    epaint::RectShape, pos2,
};
use egui::containers::menu::MenuButton;
use egui_extras::{Size, StripBuilder};
use egui_phosphor_icons::icons;
use epaint::{FontFamily, Stroke};
use once_cell::sync::OnceCell;
use usvg::Tree;
use crate::storage::asset::scene::grid::GridLocation;
use crate::ui::asset_tree::AssetTree;

const SCENE_WIDGET_SIZE: f32 = 150.0;
const PREVIEW_SIZE: f32 = 120.0;
static SELECTED_SVG: &str = include_str!("selected.svg");
static SELECTED_IMAGE: OnceCell<TextureHandle> = OnceCell::new();
fn selected_image(ctx: &Context) -> TextureId {
    SELECTED_IMAGE
        .get_or_init(|| {
            let tree = Tree::from_str(SELECTED_SVG, &Default::default()).unwrap();
            let mut pixmap =
                tiny_skia::Pixmap::new(PREVIEW_TEXTURE_SIZE as u32, PREVIEW_TEXTURE_SIZE as u32)
                    .unwrap();
            resvg::render(
                &tree,
                tiny_skia::Transform::from_scale(
                    f32::from(PREVIEW_TEXTURE_SIZE) / tree.size().width(),
                    f32::from(PREVIEW_TEXTURE_SIZE) / tree.size().height(),
                ),
                &mut pixmap.as_mut(),
            );
            let image = ColorImage::from_rgba_unmultiplied(
                [PREVIEW_TEXTURE_SIZE as usize; 2],
                pixmap.data(),
            );
            ctx.load_texture("selected_scene_image", image, TextureOptions::default())
        })
        .id()
}

pub struct SceneInstanceWidget<'a> {
    pub selected_scene_instance: &'a mut SceneInstancePathId,
    pub deck_path: DeckPath,
    pub scene_instance: &'a mut SceneInstance,
    pub groups: &'a Groups,
    pub svg: Option<TextureHandle>,
    pub size: Vec2,
    pub timing: &'a Timing,
}

impl Widget for SceneInstanceWidget<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        // a read lock to the scene data must be obtained
        // No poisoning of the lock is assumed here, as panics are not handled currently
        // and are unrecoverable anyway
        let active = self.scene_instance.active;
        let preview_color = self.scene_instance.color;
        let name = self.scene_instance.id.to_string();

        let mut beat_progression = self.timing.beat_progression();

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
            .outer_margin(10)
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

                // content strip grid
                StripBuilder::new(ui)
                    .size(Size::exact(20.0))
                    // little space between the name and the preview
                    .size(Size::exact(5.0))
                    .size(Size::remainder())
                    .cell_layout(Layout::top_down(Align::Min))
                    .vertical(|mut strip| {
                        // name and Play button
                        strip.strip(|builder| {
                            builder
                                .size(Size::relative(0.6))
                                .size(Size::remainder())
                                .horizontal(|mut strip| {
                                    strip.cell(|ui| {
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
                                        let (galley_pos, galley, _) = title_label.layout_in_ui(ui);
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
                                    });
                                    strip.cell(|ui| {
                                        let button_response = ui.add_sized(
                                            Vec2 {
                                                x: ui.available_width(),
                                                y: ui.available_height(),
                                            },
                                            Button::new(if active {
                                                icons::PAUSE.fill().color(Color32::GREEN).size(16.0)
                                            } else {
                                                icons::PLAY.fill().color(Color32::GREEN).size(16.0)
                                            })
                                                .stroke(Stroke::new(0.3, Color32::WHITE))
                                                .sense(Sense::drag()),
                                        );
                                        // this is a workaround for https://github.com/emilk/egui/issues/7767
                                        if button_response.drag_started()
                                            || button_response.clicked()
                                        {
                                            self.scene_instance.active =
                                                !self.scene_instance.active;
                                        }
                                    });
                                });
                        }); // end name/Play button strip

                        strip.empty();
                        // preview/dimmer placeholder
                        strip.strip(|builder| {
                            builder
                                .size(Size::exact(PREVIEW_SIZE))
                                .size(Size::remainder())
                                .cell_layout(Layout::top_down(Align::Min))
                                .horizontal(|mut strip| {
                                    strip.cell(|ui| {
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
                                                            Rect::from_min_max(
                                                                pos2(0.0, 0.0),
                                                                pos2(1.0, 1.0),
                                                            ),
                                                        ),
                                                ));
                                            }
                                        });
                                    });
                                    strip.cell(|ui| {
                                        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                                            egui::Frame::default().show(ui, |ui| {
                                                ui.take_available_height();
                                                ui.set_width(20.0);

                                                let dimmer = self.scene_instance.input_dimmer
                                                    * self
                                                    .scene_instance
                                                    .opacity
                                                    .value(beat_progression);

                                                ui.add(GledSlider {
                                                    real_value: if self.scene_instance.active {
                                                        dimmer
                                                    } else {
                                                        0.0
                                                    },
                                                    size: 20.0,
                                                    max_value: 100.0,
                                                    value: &mut self
                                                        .scene_instance
                                                        .opacity
                                                        .multiplier(),
                                                    show_label: false,
                                                    horizontal: false,
                                                });
                                            });
                                        });
                                    });
                                });
                        }); // end outer placeholder/dimmer preview strip
                    }); // end outer vertical strip
            }) // end outer Frame::show
            .response
    }
}

pub struct EmptyGridSpot<'a> {
    pub(crate) project: &'a mut Project,
    pub location: GridLocation,
}

impl Widget for EmptyGridSpot<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        Frame::default()
            .fill(Color32::from_gray(50))
            .stroke(Stroke {
                width: 0.3,
                color: Color32::WHITE,
            })
            .corner_radius(2)
            .inner_margin(14)
            .outer_margin(10)
            .show(ui, |ui|
                {
                    MenuButton::from_button(
                        Button::new(
                            RichText::new("+")
                                .family(FontFamily::Monospace)
                                .size(60.0)
                                .color(Color32::from_gray(120))
                        ).min_size(Vec2{x: SCENE_WIDGET_SIZE, y: SCENE_WIDGET_SIZE})
                    )
                        .ui(ui,
                            // show the plus in the middle
                            |ui| {
                                let scene = AssetTree::<Scene>::show_asset_selection(ui, ui.make_persistent_id("SelectSceneForEmptyGridSpot"));
                                if let Some(scene) = scene {
                                    self.project.add_scene(DeckPath::Grid, scene);
                                    self.project.init_gpu();
                                }
                            }
                        )
                }).response
    }
}