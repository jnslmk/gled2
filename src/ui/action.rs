use crate::pipeline::group::Group;
use crate::{
    app::{App, svg::Svg},
    input::artnet::ARTNET_CONFIG,
    input::event::InputEvent,
    storage::{
        asset::{
            Asset,
            animation::argument::{Argument, ArgumentKind, ArgumentKindId},
            animation::{Animation, config::float_value::FloatValue},
            curve::{Curve, multiplied_curve::MultipliedCurve},
            palette::{Color as PaletteColor, Palette},
            project::{Project, scene_instance_path::SceneInstanceUnion},
            scene::{
                Scene, color::SceneInstanceColor, effect::Effect, grid::GridLocation,
                instance::SceneInstance,
            },
        },
        asset_id::AssetId,
        collections::Collections,
    },
};
use cpal::DeviceId;
use egui::ViewportId;
use kanal::{Receiver, Sender, unbounded};
use notify_rust::Notification;
use once_cell::sync::OnceCell;
use std::sync::Arc;

static ACTION_SENDER: OnceCell<Sender<UiAction>> = OnceCell::new();

mod handle_ui_actions;

#[derive(Debug)]
pub enum UiAction {
    AddScene(GridLocation, AssetId<Scene>),
    SetProject(AssetId<Project>),
    SelectScene(SceneInstanceUnion),
    SelectSceneByLocation(GridLocation),
    DeleteSceneInstance {
        location: GridLocation,
    },
    DeleteSceneInstancePath(SceneInstanceUnion),
    CloneSceneInstance(GridLocation),
    CloneSceneInstancePath(SceneInstanceUnion),
    SendPositions,
    ReloadShaderCode(Option<AssetId<Animation>>),
    CloseWindow(ViewportId),
    SetSvg(Option<Svg>),
    OpenGitConfigWindow,
    Tap,
    SpeedAdd(f32),
    SpeedMultiply(f32),
    SetBlackout(bool),
    /// Set opacity for a scene instance
    ///
    /// This also activates the scene instance
    SetSceneOpacity(SceneInstanceUnion, f32),
    ToggleSceneActive(SceneInstanceUnion),
    SetSceneActive(SceneInstanceUnion, bool),
    SetSceneFlash(SceneInstanceUnion, bool),
    SetMainDimmer(f32),
    MidiOutputActive(bool),
    SwapScenes(GridLocation, GridLocation),
    SetSceneName(SceneInstanceUnion, String),
    SetSceneColor(SceneInstanceUnion, SceneInstanceColor),
    SetSceneInputDimmer(SceneInstanceUnion, f32),
    SetSceneIgnoreMainDimmer(SceneInstanceUnion, bool),
    SetSceneBeatOffset(SceneInstanceUnion, f32),
    SetSceneSetOffsetOnFlash(SceneInstanceUnion, bool),
    /// Set only the multiplier of the opacity curve (preserves the curve asset)
    SetSceneOpacityMultiplier(SceneInstanceUnion, f32),
    /// Set only the curve asset for the opacity (preserves the multiplier)
    SetSceneOpacityCurve(SceneInstanceUnion, Option<AssetId<Curve>>),
    /// Set only the multiplier of the beat_offset curve (preserves the curve asset)
    SetSceneBeatOffsetMultiplier(SceneInstanceUnion, f32),
    /// Set only the curve asset for beat_offset (preserves the multiplier)
    SetSceneBeatOffsetCurve(SceneInstanceUnion, Option<AssetId<Curve>>),
    SetSceneActivationInput(SceneInstanceUnion, Option<InputEvent>),
    SetSceneFlashInput(SceneInstanceUnion, Option<InputEvent>),
    SetSceneDimmerInput(SceneInstanceUnion, Option<InputEvent>),
    SetScenePaletteOverwrite(SceneInstanceUnion, Option<Option<Palette>>),
    SetScenePaletteOverwriteFromAsset(SceneInstanceUnion, Option<Option<AssetId<Palette>>>),
    SetScenePaletteOverwritePrimary(SceneInstanceUnion, [f32; 3]),
    SetScenePaletteOverwriteSecondary(SceneInstanceUnion, [f32; 3]),
    SetScenePaletteOverwriteGradient(SceneInstanceUnion, usize, [f32; 3]),
    ClearSceneGroupsOverwrite(SceneInstanceUnion),
    SetSceneGroupsOverwriteEntry(SceneInstanceUnion, usize, String),
    RemoveSceneGroupsOverwriteEntry(SceneInstanceUnion, usize),
    SetProjectPalette(Option<Palette>),
    SetProjectPaletteFromAsset(Option<AssetId<Palette>>),
    SetProjectPalettePrimary([f32; 3]),
    SetProjectPaletteSecondary([f32; 3]),
    SetProjectPaletteGradient(usize, [f32; 3]),
    SetProjectArtnetControlActive(bool),
    SetProjectArtnetControlUniverse(u16),
    SetProjectGroup(usize, String),
    RemoveProjectGroup(usize),
    ClearProjectGroups,
    AddProjectTapInputArtnet(u16),
    RemoveProjectTapInputArtnet(u16),
    ClearProjectTapInput,
    AddProjectBlackoutInputArtnet(u16),
    RemoveProjectBlackoutInputArtnet(u16),
    ClearProjectBlackoutInput,
    AddProjectBlackoutHoldInputArtnet(u16),
    RemoveProjectBlackoutHoldInputArtnet(u16),
    ClearProjectBlackoutHoldInput,
    AddProjectHalfInputArtnet(u16),
    RemoveProjectHalfInputArtnet(u16),
    ClearProjectHalfInput,
    AddProjectDoubleInputArtnet(u16),
    RemoveProjectDoubleInputArtnet(u16),
    ClearProjectDoubleInput,
    SetSceneEffectOpacity(SceneInstanceUnion, usize, f32),
    SetSceneEffectOpacityMultiplier(SceneInstanceUnion, usize, f32),
    SetSceneEffectOpacityCurve(SceneInstanceUnion, usize, Option<AssetId<Curve>>),
    SetSceneEffectColorShift(SceneInstanceUnion, usize, f32),
    SetSceneEffectColorShiftMultiplier(SceneInstanceUnion, usize, f32),
    SetSceneEffectColorShiftCurve(SceneInstanceUnion, usize, Option<AssetId<Curve>>),
    SetSceneEffectBeatProgression(SceneInstanceUnion, usize, f32),
    SetSceneEffectBeatProgressionMultiplier(SceneInstanceUnion, usize, f32),
    SetSceneEffectBeatProgressionCurve(SceneInstanceUnion, usize, Option<AssetId<Curve>>),
    SetSceneEffectBeatOffset(SceneInstanceUnion, usize, f32),
    SetSceneEffectBeatOffsetMultiplier(SceneInstanceUnion, usize, f32),
    SetSceneEffectBeatOffsetCurve(SceneInstanceUnion, usize, Option<AssetId<Curve>>),
    SetSceneEffectSpeedExponent(SceneInstanceUnion, usize, i32),
    SetSceneEffectGroupIndex(SceneInstanceUnion, usize, usize),
    SetSceneEffectAnimation(SceneInstanceUnion, usize, Option<AssetId<Animation>>),
    SetSceneEffectAnimationConfigU32(SceneInstanceUnion, usize, usize, u32),
    SetSceneEffectAnimationConfigF32(SceneInstanceUnion, usize, usize, f32),
    AddSceneEffect(SceneInstanceUnion),
    RemoveSceneEffect(SceneInstanceUnion, usize),
    CloneSceneEffect(SceneInstanceUnion, usize),
    SetAnimationShaderCode(AssetId<Animation>, String),
    AddAnimationArgument(AssetId<Animation>),
    RemoveAnimationArgument(AssetId<Animation>, usize),
    SetAnimationArgumentName(AssetId<Animation>, usize, String),
    SetAnimationArgumentKind(AssetId<Animation>, usize, ArgumentKindId),
    Error(String),
    SetAudioDevice(Option<DeviceId>),
}

fn scene_effect_by_target(
    project: &mut Project,
    target: SceneInstanceUnion,
    selected_scene_location: GridLocation,
    effect_index: usize,
) -> Option<&mut Effect> {
    scene_instance_by_target(project, target, selected_scene_location)?
        .scene
        .effect(effect_index)
}

fn location_by_target(
    project: &mut Project,
    target: SceneInstanceUnion,
    selected_scene_location: GridLocation,
) -> Option<GridLocation> {
    match target {
        SceneInstanceUnion::Selected => Some(selected_scene_location),
        SceneInstanceUnion::Grid(location) => Some(location),
        SceneInstanceUnion::Quick(index) => {
            project.location_by_location_or_quick_index(SceneInstanceUnion::Quick(index))
        }
    }
}

fn scene_instance_by_target(
    project: &mut Project,
    target: SceneInstanceUnion,
    selected_scene_location: GridLocation,
) -> Option<&mut SceneInstance> {
    let location = location_by_target(project, target, selected_scene_location)?;
    project.get_scenes_instance(&location)
}

fn set_scene_opacity(scene_instance: &mut SceneInstance, opacity: f32) {
    scene_instance.active = true;
    scene_instance.opacity = MultipliedCurve::new_multiplier(opacity);
}

fn apply_palette_action(
    project: &mut Project,
    selected_scene_instance: GridLocation,
    collections: &Collections,
    action: &UiAction,
) -> bool {
    match action {
        UiAction::SetProjectPalette(palette) => {
            project.palette = palette.clone();
            true
        }
        UiAction::SetProjectPaletteFromAsset(palette_id) => {
            project.palette = palette_id
                .and_then(|palette_id| Asset::get(palette_id, collections))
                .map(|palette| palette.data.clone());
            true
        }
        UiAction::SetProjectPalettePrimary(rgb) => {
            set_palette_primary(project.palette.get_or_insert_with(Palette::default), *rgb);
            true
        }
        UiAction::SetProjectPaletteSecondary(rgb) => {
            set_palette_secondary(project.palette.get_or_insert_with(Palette::default), *rgb);
            true
        }
        UiAction::SetProjectPaletteGradient(gradient_index, rgb) => {
            set_palette_gradient(
                project.palette.get_or_insert_with(Palette::default),
                *gradient_index,
                *rgb,
            );
            true
        }
        UiAction::SetScenePaletteOverwrite(path, palette_overwrite) => {
            if let Some(scene_instance) =
                scene_instance_by_target(project, *path, selected_scene_instance)
            {
                scene_instance.palette_overwrite = palette_overwrite.clone();
            }
            true
        }
        UiAction::SetScenePaletteOverwriteFromAsset(path, palette_overwrite) => {
            if let Some(scene_instance) =
                scene_instance_by_target(project, *path, selected_scene_instance)
            {
                scene_instance.palette_overwrite = palette_overwrite.map(|palette_overwrite| {
                    palette_overwrite.and_then(|palette_id| {
                        Asset::get(palette_id, collections).map(|palette| palette.data.clone())
                    })
                });
            }
            true
        }
        UiAction::SetScenePaletteOverwritePrimary(path, rgb) => {
            if let Some(scene_instance) =
                scene_instance_by_target(project, *path, selected_scene_instance)
            {
                set_palette_primary(scene_palette_overwrite_mut(scene_instance), *rgb);
            }
            true
        }
        UiAction::SetScenePaletteOverwriteSecondary(path, rgb) => {
            if let Some(scene_instance) =
                scene_instance_by_target(project, *path, selected_scene_instance)
            {
                set_palette_secondary(scene_palette_overwrite_mut(scene_instance), *rgb);
            }
            true
        }
        UiAction::SetScenePaletteOverwriteGradient(path, gradient_index, rgb) => {
            if let Some(scene_instance) =
                scene_instance_by_target(project, *path, selected_scene_instance)
            {
                set_palette_gradient(
                    scene_palette_overwrite_mut(scene_instance),
                    *gradient_index,
                    *rgb,
                );
            }
            true
        }
        _ => false,
    }
}

fn scene_palette_overwrite_mut(scene_instance: &mut SceneInstance) -> &mut Palette {
    scene_instance
        .palette_overwrite
        .get_or_insert_with(|| Some(Palette::default()))
        .get_or_insert_with(Palette::default)
}

fn set_palette_primary(palette: &mut Palette, rgb: [f32; 3]) {
    palette.primary = PaletteColor::new(rgb[0], rgb[1], rgb[2]);
}

fn set_palette_secondary(palette: &mut Palette, rgb: [f32; 3]) {
    palette.secondary = PaletteColor::new(rgb[0], rgb[1], rgb[2]);
}

fn set_palette_gradient(palette: &mut Palette, gradient_index: usize, rgb: [f32; 3]) {
    if let Some(color) = palette.gradient.get_mut(gradient_index) {
        *color = PaletteColor::new(rgb[0], rgb[1], rgb[2]);
    }
}

fn update_animation(
    collections: &mut Collections,
    animation_id: AssetId<Animation>,
    apply: impl FnOnce(&mut Animation),
) {
    if let Some(animation) = Asset::get(animation_id, collections) {
        let mut animation = Arc::unwrap_or_clone(animation);
        apply(&mut animation.data);
        animation.save(collections);
    }
}

impl UiAction {
    pub fn init_queue() -> Receiver<UiAction> {
        let (sender, receiver) = unbounded();
        ACTION_SENDER.set(sender).unwrap();
        receiver
    }

    pub fn enqueue(self) {
        ACTION_SENDER
            .get()
            .expect("Action sender not initialized")
            .send(self)
            .expect("Action receiver dropped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_scene_instance() -> SceneInstance {
        SceneInstance::from_scene_id(AssetId::default(), &Collections::default())
    }

    #[test]
    fn project_palette_actions_create_and_update_local_palette() {
        let mut project = Project::default();

        assert!(apply_palette_action(
            &mut project,
            GridLocation::default(),
            &Collections::default(),
            &UiAction::SetProjectPalettePrimary([0.1, 0.2, 0.3]),
        ));
        assert!(apply_palette_action(
            &mut project,
            GridLocation::default(),
            &Collections::default(),
            &UiAction::SetProjectPaletteGradient(2, [0.4, 0.5, 0.6]),
        ));

        let palette = project.palette.expect("project palette should be created");
        assert_eq!(palette.primary.rgb(), [0.1, 0.2, 0.3]);
        assert_eq!(palette.gradient[2].rgb(), [0.4, 0.5, 0.6]);
    }

    #[test]
    fn location_by_target_resolves_selected_scene() {
        let mut project = Project::default();
        let selected = GridLocation::new(1, 1);

        assert_eq!(
            location_by_target(&mut project, SceneInstanceUnion::Selected, selected),
            Some(selected)
        );
    }

    #[test]
    fn set_scene_opacity_activates_scene_instance() {
        let mut scene_instance = test_scene_instance();

        assert!(!scene_instance.active);
        set_scene_opacity(&mut scene_instance, 0.35);

        assert!(scene_instance.active);
        assert_eq!(scene_instance.opacity.multiplier, 0.35);
    }

    #[test]
    fn selected_scene_palette_actions_create_local_override() {
        let mut project = Project::default();
        let selected = GridLocation::new(1, 1);
        project.add_scene_instance(selected, test_scene_instance());

        assert!(apply_palette_action(
            &mut project,
            selected,
            &Collections::default(),
            &UiAction::SetScenePaletteOverwritePrimary(
                SceneInstanceUnion::Selected,
                [0.2, 0.3, 0.4],
            ),
        ));
        assert!(apply_palette_action(
            &mut project,
            selected,
            &Collections::default(),
            &UiAction::SetScenePaletteOverwriteGradient(
                SceneInstanceUnion::Selected,
                5,
                [0.7, 0.8, 0.9],
            ),
        ));

        let scene_instance = project
            .get_scenes_instance(&selected)
            .expect("selected scene should exist");
        let palette = scene_instance
            .palette_overwrite
            .as_ref()
            .and_then(|palette| palette.as_ref())
            .expect("selected scene should have a local override");
        assert_eq!(palette.primary.rgb(), [0.2, 0.3, 0.4]);
        assert_eq!(palette.gradient[5].rgb(), [0.7, 0.8, 0.9]);
    }

    #[test]
    fn direct_scene_palette_edit_replaces_explicit_none_with_palette() {
        let mut project = Project::default();
        let location = GridLocation::new(2, 3);
        let mut scene_instance = test_scene_instance();
        scene_instance.palette_overwrite = Some(None);
        project.add_scene_instance(location, scene_instance);

        assert!(apply_palette_action(
            &mut project,
            GridLocation::default(),
            &Collections::default(),
            &UiAction::SetScenePaletteOverwriteSecondary(
                SceneInstanceUnion::Grid(location),
                [0.9, 0.8, 0.7],
            ),
        ));

        let scene_instance = project
            .get_scenes_instance(&location)
            .expect("scene should exist");
        let palette = scene_instance
            .palette_overwrite
            .as_ref()
            .and_then(|palette| palette.as_ref())
            .expect("edit should create a palette override");
        assert_eq!(palette.secondary.rgb(), [0.9, 0.8, 0.7]);
    }
}
