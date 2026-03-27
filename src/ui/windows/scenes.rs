use crate::{
    app::{persistant_state::PersistantState, timing::Timing},
    audio::sound_data::SoundData,
    pipeline::renderer_callback::RendererCallback,
    storage::{
        asset::{
            Asset,
            animation::Animation,
            scene::Scene,
        },
        asset_id::AssetId,
        collections::Collections,
    },
    ui::{
        asset::CollectionsChangeButton,
        asset_tree::{AssetTree, TREE_WIDTH, TreeSelection},
        scene_effect_editor::{
            SceneEffectEditorState, scene_effect_list_ui, selected_effect_editor_ui,
        },
        window_common::{default_viewport_builder, gled_window_frame},
    },
    wgpu_render_state,
};
use egui::{
    Color32, Context, Id, Margin, ScrollArea, Stroke, Vec2, ViewportId,
    scroll_area::ScrollBarVisibility::AlwaysVisible,
};
use wgpu::CommandEncoderDescriptor;

#[derive(Default)]
pub struct ScenesWindow {
    open: bool,
    dirty: bool,
    tree: AssetTree<Scene>,
    effect_editor: SceneEffectEditorState,
}

impl ScenesWindow {
    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn update(
        &mut self,
        ctx: &Context,
        timing: &Timing,
        collections: &mut Collections,
        persistant_state: &mut PersistantState,
        sound_data: &mut SoundData,
    ) {
        if !self.open {
            return;
        }
        if !self.open {
            return;
        }

        ctx.show_viewport_immediate(
            ViewportId(Id::new("scenes window")),
            default_viewport_builder()
                .with_inner_size(Vec2::new(950.0, 500.0))
                .with_min_inner_size(Vec2::new(950.0, 500.0)),
            |ctx, _viewport_class| {
                ctx.input(|input| {
                    if input.viewport().close_requested() {
                        self.open = false;
                    }
                });

                gled_window_frame(ctx, "Scenes", |ui| {
                    egui::SidePanel::left("scenes tree")
                        .exact_width(TREE_WIDTH)
                        .resizable(false)
                        .show_inside(ui, |ui| {
                            if self
                                .tree
                                .show(ui, ui.make_persistent_id("scenes_tree"), collections)
                            {
                                self.dirty = false;
                                self.effect_editor.reset();
                            }
                        });

                    egui::SidePanel::right("scene editor")
                        .exact_width(300.0)
                        .resizable(false)
                        .show_inside(ui, |ui| {
                            let TreeSelection::Asset(scene) = self.tree.selected() else {
                                return;
                            };

                            ScrollArea::vertical()
                                .id_salt("scene_editor_scroll")
                                .scroll_bar_visibility(AlwaysVisible)
                                .show(ui, |ui| {
                                    self.dirty |= selected_effect_editor_ui(
                                        ui,
                                        &mut scene.data,
                                        &mut self.effect_editor,
                                        true,
                                        None,
                                        1.0,
                                        collections,
                                        sound_data,
                                    );
                                });
                        });

                    egui::CentralPanel::default().show_inside(ui, |ui| {
                        let asset_changed: bool =
                            self.tree.common_settings(ui, &mut self.dirty, collections);

                        ui.add_space(4.0);

                        if let TreeSelection::Asset(asset) = &mut self.tree.selected() {
                            let mut scene = asset.data.clone();
                            if asset_changed
                                && let Some(effect) =
                                    scene.effect(self.effect_editor.selected_effect)
                            {
                                effect
                                    .state
                                    .set_shader_code(&effect.shader_code_complete(collections));
                            }

                            egui::Frame::NONE
                                .inner_margin(Margin::from(6.0))
                                .stroke(Stroke::new(1.0, Color32::DARK_GRAY))
                                .show(ui, |ui| {
                                    ui.label("Preview Palette");
                                    ui.vertical_centered_justified(|ui| {
                                        if persistant_state
                                            .preview_palette_mut()
                                            .collections_change_button(ui, collections)
                                        {
                                            persistant_state.save();
                                        }
                                    });
                                });

                            ui.add_space(4.0);

                            {
                                for effect in scene.effects.iter_mut() {
                                    effect.state.beat_progression = timing.beat_progression();
                                    effect.state.beats_per_minute = timing.beats_per_minute();
                                    effect.state.framerate = timing.framerate().unwrap_or_default();
                                }

                                let wgpu_render_state = wgpu_render_state();
                                let device = wgpu_render_state.device;
                                let queue = &wgpu_render_state.queue;
                                scene.prepare(
                                    queue,
                                    persistant_state
                                        .preview_palette()
                                        .and_then(|id| Asset::get(id, collections))
                                        .map(|palette| palette.data.clone()),
                                    &Default::default(),
                                    1.0,
                                    collections,
                                    sound_data,
                                );
                                let mut encoder =
                                    device.create_command_encoder(&CommandEncoderDescriptor {
                                        label: Some("Render animations for scene editor"),
                                    });
                                scene.render(&mut encoder, false);
                                RendererCallback::add(encoder.finish());
                            }

                            ScrollArea::vertical()
                                .id_salt("effects_scroll")
                                .auto_shrink([false, false])
                                .scroll_bar_visibility(AlwaysVisible)
                                .show(ui, |ui| {
                                    ui.set_max_width(ui.available_width() - 30.0);
                                    if let TreeSelection::Asset(scene) = self.tree.selected() {
                                        scene_effect_list_ui(
                                            ui,
                                            &scene.data,
                                            &mut self.effect_editor,
                                            true,
                                            None,
                                            None,
                                            false,
                                            None,
                                            collections,
                                            sound_data,
                                        );
                                    }
                                });
                        }
                    });
                });
            },
        );
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn reload_shader_code(
        &mut self,
        animation: Option<AssetId<Animation>>,
        collections: &mut Collections,
    ) {
        let TreeSelection::Asset(scene) = self.tree.selected() else {
            return;
        };

        scene.data.reload_shader_code(animation, collections);
    }
}
