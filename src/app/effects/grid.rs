use super::App;
use crate::pipeline::constants::PREVIEW_TEXTURE_SIZE;
use crate::storage::asset::scene::grid::GridLocation;
use crate::ui::scene_instance::dnd::{dnd_drag_source, dnd_drop_zone};
use crate::ui::scene_instance::widget::{EmptyGridSpot, SCENE_WIDGET_SIZE};
use crate::ui::scene_instance::widget::SceneInstanceWidget;
use crate::{app::PersistantState, storage::asset::project::DeckPath};
use egui::{
    Color32, Context, Frame, Grid, Id, Sense, TextureHandle, Ui, UiBuilder, Vec2,
    scroll_area::ScrollBarVisibility::AlwaysVisible,
};
use emath::{Rect, pos2, vec2};
use epaint::textures::TextureOptions;
use epaint::{ColorImage, Stroke, StrokeKind, TextureId};
use once_cell::sync::OnceCell;
use usvg::Tree;

pub const WIDTH: usize = 6;
pub const HEIGHT: usize = 4;

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

impl App {
    pub fn effects_grid(&mut self, ui: &mut Ui, svg: Option<TextureHandle>) {
        egui::ScrollArea::both()
            .id_salt("grid scroll")
            .auto_shrink([false, false])
            .scroll_bar_visibility(AlwaysVisible)
            .show(ui, |ui| {
                let Some(project) = self.project.as_mut() else {
                    return;
                };
                let to_global = ui.ctx().layer_transform_to_global(ui.layer_id()).unwrap_or_default();
                let deck_path = DeckPath::Grid;
                let effects_size = PersistantState::effects_size();
                let groups = project.groups.clone();

                let mut grid = match deck_path {
                    DeckPath::Grid => &mut project.scenes_instances_grid,
                    DeckPath::Quick => &mut project.scenes_instances_quick,
                };
                let start_pos = ui.cursor().min + vec2(20., 20.);

                    for row in 0..HEIGHT {
                        for col in 0..WIDTH {
                            let location = GridLocation { col, row };

                            let tile_length = SCENE_WIDGET_SIZE + 28.;
                            let rect = to_global.mul_rect
                            (Rect::from_min_size(
                                start_pos + vec2(col as f32 * (tile_length + 20.), row as f32 * (tile_length + 20.)),
                                vec2(tile_length, tile_length )));
                            let (_, dropped_payload) = ui.scope_builder(UiBuilder::new().max_rect(rect),|ui| {
                                    dnd_drop_zone::<GridLocation, ()>(ui, Frame::default().corner_radius(2.), |ui| {
                                        let scene = grid.get_mut(&location);
                                        match scene {
                                            Some(scene_instance) => {
                                                let item_id = Id::new((
                                                    "Draggable Scene Widget",
                                                    scene_instance.id,
                                                ));
                                                let dnd_response =
                                                    dnd_drag_source(ui, item_id, location, |ui| {
                                                        ui.add(SceneInstanceWidget {
                                                            selected_scene_instance: &mut self
                                                                .selected_scene_instance,
                                                            deck_path,
                                                            scene_instance,
                                                            svg: svg.clone(),
                                                            size: Vec2::splat(effects_size),
                                                            groups: &groups,
                                                            timing: &self.timing,
                                                        })
                                                    })
                                                    .response;

                                                if self.selected_scene_instance.id
                                                    == scene_instance.id
                                                {
                                                    ui.painter().image(
                                                        selected_image(ui.ctx()),
                                                        dnd_response.rect,
                                                        Rect::from_min_max(
                                                            pos2(0.0, 0.0),
                                                            pos2(1.0, 1.0),
                                                        ),
                                                        Color32::WHITE,
                                                    );
                                                }
                                                if dnd_response.hovered() {
                                                    ui.painter().rect_stroke(
                                                        dnd_response.rect,
                                                        5.0,
                                                        Stroke::new(2., Color32::WHITE),
                                                        StrokeKind::Outside,
                                                    );
                                                }
                                            } // Some(scene) =>
                                            None => {
                                                let init_gpu = false;
                                                ui.add(EmptyGridSpot {
                                                    selected_scene_instance: &mut self
                                                        .selected_scene_instance,
                                                    grid: &mut grid,
                                                    location,
                                                    init_gpu: &init_gpu,
                                                });
                                            }
                                        };
                                    })
                                })
                                .inner; // dnd_drop_zone
                            if let Some(dragged_payload) = dropped_payload {
                                // The user dropped onto this cell
                                let from = GridLocation {
                                    row: dragged_payload.row,
                                    col: dragged_payload.col,
                                };
                                let to = GridLocation { row, col };

                                // Ownership of the scene instances must temporarily be taken to swap
                                let to_item = grid.remove(&to);
                                let from_item = { grid.remove(&from) };
                                if let Some(from_item) = from_item {
                                    grid.insert(to, from_item);
                                }
                                // reinsert the to item to the from location
                                if let Some(to_item) = to_item {
                                    grid.insert(from, to_item);
                                }
                            }
                        }
                    } // grid
            });
    }
}
