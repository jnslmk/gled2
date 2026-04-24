use log::warn;
use rosc::{OscMessage, OscType};
use uuid::Uuid;

use crate::input::event::InputEvent;
use crate::storage::asset::animation::argument::ArgumentKindId;
use crate::storage::asset::project::scene_instance_path::{
    QuickSceneInstanceIndex, SceneInstanceUnion, selected_scene_instance_index,
};
use crate::storage::asset::scene::Scene;
use crate::storage::asset::scene::color::SceneInstanceColor;
use crate::storage::asset::scene::grid::GridLocation;
use crate::storage::asset::{animation::Animation, curve::Curve, palette::Palette};
use crate::storage::asset_id::AssetId;
use crate::ui::action::UiAction;

pub(super) fn parse_message(msg: &OscMessage) -> Result<UiAction, ()> {
    let path: Vec<&str> = msg
        .addr
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    match path.as_slice() {
        ["project", "main_dimmer"] => Ok(UiAction::SetMainDimmer(parse_f32(msg, 0)?)),
        ["project", "blackout"] => Ok(UiAction::SetBlackout(parse_bool(msg, 0)?)),
        ["project", "palette"] => Ok(UiAction::SetProjectPaletteFromAsset(
            parse_optional_palette_id(msg, 0)?,
        )),
        ["project", "palette", "primary"] => {
            Ok(UiAction::SetProjectPalettePrimary(parse_rgb(msg)?))
        }
        ["project", "palette", "secondary"] => {
            Ok(UiAction::SetProjectPaletteSecondary(parse_rgb(msg)?))
        }
        ["project", "palette", "gradient", gradient_index] => Ok(
            UiAction::SetProjectPaletteGradient(parse_index(gradient_index)?, parse_rgb(msg)?),
        ),
        ["project", "artnet_control", "active"] => {
            Ok(UiAction::SetProjectArtnetControlActive(parse_bool(msg, 0)?))
        }
        ["project", "artnet_control", "universe"] => Ok(UiAction::SetProjectArtnetControlUniverse(
            parse_u16(msg, 0)?,
        )),
        ["project", "groups", "clear"] => Ok(UiAction::ClearProjectGroups),
        ["project", "groups", "remove", group_index] => {
            Ok(UiAction::RemoveProjectGroup(parse_index(group_index)?))
        }
        ["project", "groups", group_index] => Ok(UiAction::SetProjectGroup(
            parse_index(group_index)?,
            parse_string(msg, 0)?,
        )),
        ["project", "input", event_kind, "clear"] => parse_project_input_clear(event_kind),
        ["project", "input", event_kind, "add", "artnet"] => {
            parse_project_input_add_artnet(event_kind, parse_u16(msg, 0)?)
        }
        ["project", "input", event_kind, "remove", "artnet"] => {
            parse_project_input_remove_artnet(event_kind, parse_u16(msg, 0)?)
        }

        ["asset", "animation", animation_id, "shader_code"] => {
            Ok(UiAction::SetAnimationShaderCode(
                parse_animation_id_from_path(animation_id)?,
                parse_string(msg, 0)?,
            ))
        }
        ["asset", "animation", animation_id, "argument", "add"] => Ok(
            UiAction::AddAnimationArgument(parse_animation_id_from_path(animation_id)?),
        ),
        [
            "asset",
            "animation",
            animation_id,
            "argument",
            argument_index,
            "remove",
        ] => Ok(UiAction::RemoveAnimationArgument(
            parse_animation_id_from_path(animation_id)?,
            parse_index(argument_index)?,
        )),
        [
            "asset",
            "animation",
            animation_id,
            "argument",
            argument_index,
            "name",
        ] => Ok(UiAction::SetAnimationArgumentName(
            parse_animation_id_from_path(animation_id)?,
            parse_index(argument_index)?,
            parse_string(msg, 0)?,
        )),
        [
            "asset",
            "animation",
            animation_id,
            "argument",
            argument_index,
            "kind",
        ] => Ok(UiAction::SetAnimationArgumentKind(
            parse_animation_id_from_path(animation_id)?,
            parse_index(argument_index)?,
            parse_argument_kind(msg, 0)?,
        )),

        ["timing", "tap"] => Ok(UiAction::Tap),
        ["timing", "speed", "add"] => Ok(UiAction::SpeedAdd(parse_f32(msg, 0)?)),
        ["timing", "speed", "multiply"] => Ok(UiAction::SpeedMultiply(parse_f32(msg, 0)?)),

        ["scene", "selected", "effect", "add"] => {
            Ok(UiAction::AddSceneEffect(selected_scene_instance_index()))
        }
        ["scene", "selected", "effect", effect_index, action] => {
            let effect_index = parse_index(effect_index)?;
            parse_target_effect_action(msg, selected_scene_instance_index(), effect_index, action)
        }
        ["scene", "selected", "effect", effect_index, curve_field, sub_action]
            if matches!(
                *curve_field,
                "opacity" | "color_shift" | "beat_progression" | "beat_offset"
            ) =>
        {
            let effect_index = parse_index(effect_index)?;
            parse_target_effect_curve_action(
                msg,
                selected_scene_instance_index(),
                effect_index,
                curve_field,
                sub_action,
            )
        }
        [
            "scene",
            "selected",
            "effect",
            effect_index,
            "config",
            "u32",
            config_index,
        ] => Ok(UiAction::SetSceneEffectAnimationConfigU32(
            selected_scene_instance_index(),
            parse_index(effect_index)?,
            parse_index(config_index)?,
            parse_u32(msg, 0)?,
        )),
        [
            "scene",
            "selected",
            "effect",
            effect_index,
            "config",
            "f32",
            config_index,
        ] => Ok(UiAction::SetSceneEffectAnimationConfigF32(
            selected_scene_instance_index(),
            parse_index(effect_index)?,
            parse_index(config_index)?,
            parse_f32(msg, 0)?,
        )),
        ["scene", "selected", "groups_overwrite", "clear"] => {
            Ok(UiAction::ClearSceneGroupsOverwrite(selected_scene_instance_index()))
        }
        [
            "scene",
            "selected",
            "groups_overwrite",
            "remove",
            group_index,
        ] => Ok(UiAction::RemoveSceneGroupsOverwriteEntry(
            selected_scene_instance_index(),
            parse_index(group_index)?,
        )),
        ["scene", "selected", "groups_overwrite", group_index] => {
            Ok(UiAction::SetSceneGroupsOverwriteEntry(
                selected_scene_instance_index(),
                parse_index(group_index)?,
                parse_string(msg, 0)?,
            ))
        }
        ["scene", "selected", "palette_overwrite", action] => {
            parse_target_palette_overwrite_action(msg, selected_scene_instance_index(), action)
        }
        [
            "scene",
            "selected",
            "palette_overwrite",
            "gradient",
            gradient_index,
        ] => Ok(UiAction::SetScenePaletteOverwriteGradient(
            selected_scene_instance_index(),
            parse_index(gradient_index)?,
            parse_rgb(msg)?,
        )),
        ["scene", "selected", input_kind, mode]
            if matches!(
                *input_kind,
                "activation_input" | "flash_input" | "dimmer_input"
            ) =>
        {
            parse_target_input_action(msg, selected_scene_instance_index(), input_kind, mode)
        }
        ["scene", "selected", curve_field, sub_action]
            if matches!(*curve_field, "opacity" | "beat_offset") =>
        {
            parse_target_curve_action(
                msg,
                selected_scene_instance_index(),
                curve_field,
                sub_action,
            )
        }
        ["scene", "selected", action] => {
            parse_target_action(msg, selected_scene_instance_index(), action)
        }

        ["scene", "grid", col, row, "add"] => {
            let location = parse_grid_location(col, row)?;
            let scene_id = parse_scene_id(msg, 0)?;
            Ok(UiAction::AddScene(location, scene_id))
        }
        ["scene", "grid", col, row, "effect", "add"] => {
            let location = parse_grid_location(col, row)?;
            Ok(UiAction::AddSceneEffect(SceneInstanceUnion::Grid(location)))
        }
        ["scene", "quick", index, "effect", "add"] => {
            let index = parse_quick_index(index)?;
            Ok(UiAction::AddSceneEffect(SceneInstanceUnion::Quick(
                QuickSceneInstanceIndex { index },
            )))
        }
        ["scene", "grid", col, row, "effect", effect_index, action] => {
            let location = parse_grid_location(col, row)?;
            let effect_index = parse_index(effect_index)?;
            let target = SceneInstanceUnion::Grid(location);
            parse_target_effect_action(msg, target, effect_index, action)
        }
        ["scene", "grid", col, row, "effect", effect_index, curve_field, sub_action]
            if matches!(
                *curve_field,
                "opacity" | "color_shift" | "beat_progression" | "beat_offset"
            ) =>
        {
            let location = parse_grid_location(col, row)?;
            let effect_index = parse_index(effect_index)?;
            parse_target_effect_curve_action(
                msg,
                SceneInstanceUnion::Grid(location),
                effect_index,
                curve_field,
                sub_action,
            )
        }
        [
            "scene",
            "grid",
            col,
            row,
            "effect",
            effect_index,
            "config",
            "u32",
            config_index,
        ] => {
            let location = parse_grid_location(col, row)?;
            Ok(UiAction::SetSceneEffectAnimationConfigU32(
                SceneInstanceUnion::Grid(location),
                parse_index(effect_index)?,
                parse_index(config_index)?,
                parse_u32(msg, 0)?,
            ))
        }
        [
            "scene",
            "grid",
            col,
            row,
            "effect",
            effect_index,
            "config",
            "f32",
            config_index,
        ] => {
            let location = parse_grid_location(col, row)?;
            Ok(UiAction::SetSceneEffectAnimationConfigF32(
                SceneInstanceUnion::Grid(location),
                parse_index(effect_index)?,
                parse_index(config_index)?,
                parse_f32(msg, 0)?,
            ))
        }
        ["scene", "grid", col, row, "groups_overwrite", "clear"] => {
            let location = parse_grid_location(col, row)?;
            Ok(UiAction::ClearSceneGroupsOverwrite(
                SceneInstanceUnion::Grid(location),
            ))
        }
        [
            "scene",
            "grid",
            col,
            row,
            "groups_overwrite",
            "remove",
            group_index,
        ] => {
            let location = parse_grid_location(col, row)?;
            Ok(UiAction::RemoveSceneGroupsOverwriteEntry(
                SceneInstanceUnion::Grid(location),
                parse_index(group_index)?,
            ))
        }
        ["scene", "grid", col, row, "groups_overwrite", group_index] => {
            let location = parse_grid_location(col, row)?;
            Ok(UiAction::SetSceneGroupsOverwriteEntry(
                SceneInstanceUnion::Grid(location),
                parse_index(group_index)?,
                parse_string(msg, 0)?,
            ))
        }
        ["scene", "grid", col, row, "palette_overwrite", action] => {
            let location = parse_grid_location(col, row)?;
            parse_target_palette_overwrite_action(msg, SceneInstanceUnion::Grid(location), action)
        }
        [
            "scene",
            "grid",
            col,
            row,
            "palette_overwrite",
            "gradient",
            gradient_index,
        ] => {
            let location = parse_grid_location(col, row)?;
            Ok(UiAction::SetScenePaletteOverwriteGradient(
                SceneInstanceUnion::Grid(location),
                parse_index(gradient_index)?,
                parse_rgb(msg)?,
            ))
        }
        ["scene", "grid", col, row, input_kind, mode]
            if matches!(
                *input_kind,
                "activation_input" | "flash_input" | "dimmer_input"
            ) =>
        {
            let location = parse_grid_location(col, row)?;
            parse_target_input_action(msg, SceneInstanceUnion::Grid(location), input_kind, mode)
        }
        ["scene", "grid", col, row, curve_field, sub_action]
            if matches!(*curve_field, "opacity" | "beat_offset") =>
        {
            let location = parse_grid_location(col, row)?;
            parse_target_curve_action(
                msg,
                SceneInstanceUnion::Grid(location),
                curve_field,
                sub_action,
            )
        }
        ["scene", "grid", col, row, action] => {
            let location = parse_grid_location(col, row)?;
            let target = SceneInstanceUnion::Grid(location);
            parse_target_action(msg, target, action)
        }
        ["scene", "quick", index, "effect", effect_index, action] => {
            let index = parse_quick_index(index)?;
            let effect_index = parse_index(effect_index)?;
            let target = SceneInstanceUnion::Quick(QuickSceneInstanceIndex { index });
            parse_target_effect_action(msg, target, effect_index, action)
        }
        ["scene", "quick", index, "effect", effect_index, curve_field, sub_action]
            if matches!(
                *curve_field,
                "opacity" | "color_shift" | "beat_progression" | "beat_offset"
            ) =>
        {
            let index = parse_quick_index(index)?;
            let effect_index = parse_index(effect_index)?;
            parse_target_effect_curve_action(
                msg,
                SceneInstanceUnion::Quick(QuickSceneInstanceIndex { index }),
                effect_index,
                curve_field,
                sub_action,
            )
        }
        [
            "scene",
            "quick",
            index,
            "effect",
            effect_index,
            "config",
            "u32",
            config_index,
        ] => {
            let index = parse_quick_index(index)?;
            Ok(UiAction::SetSceneEffectAnimationConfigU32(
                SceneInstanceUnion::Quick(QuickSceneInstanceIndex { index }),
                parse_index(effect_index)?,
                parse_index(config_index)?,
                parse_u32(msg, 0)?,
            ))
        }
        [
            "scene",
            "quick",
            index,
            "effect",
            effect_index,
            "config",
            "f32",
            config_index,
        ] => {
            let index = parse_quick_index(index)?;
            Ok(UiAction::SetSceneEffectAnimationConfigF32(
                SceneInstanceUnion::Quick(QuickSceneInstanceIndex { index }),
                parse_index(effect_index)?,
                parse_index(config_index)?,
                parse_f32(msg, 0)?,
            ))
        }
        ["scene", "quick", index, "groups_overwrite", "clear"] => {
            let index = parse_quick_index(index)?;
            Ok(UiAction::ClearSceneGroupsOverwrite(
                SceneInstanceUnion::Quick(QuickSceneInstanceIndex { index }),
            ))
        }
        [
            "scene",
            "quick",
            index,
            "groups_overwrite",
            "remove",
            group_index,
        ] => {
            let index = parse_quick_index(index)?;
            Ok(UiAction::RemoveSceneGroupsOverwriteEntry(
                SceneInstanceUnion::Quick(QuickSceneInstanceIndex { index }),
                parse_index(group_index)?,
            ))
        }
        ["scene", "quick", index, "groups_overwrite", group_index] => {
            let index = parse_quick_index(index)?;
            Ok(UiAction::SetSceneGroupsOverwriteEntry(
                SceneInstanceUnion::Quick(QuickSceneInstanceIndex { index }),
                parse_index(group_index)?,
                parse_string(msg, 0)?,
            ))
        }
        ["scene", "quick", index, "palette_overwrite", action] => {
            let index = parse_quick_index(index)?;
            parse_target_palette_overwrite_action(
                msg,
                SceneInstanceUnion::Quick(QuickSceneInstanceIndex { index }),
                action,
            )
        }
        [
            "scene",
            "quick",
            index,
            "palette_overwrite",
            "gradient",
            gradient_index,
        ] => {
            let index = parse_quick_index(index)?;
            Ok(UiAction::SetScenePaletteOverwriteGradient(
                SceneInstanceUnion::Quick(QuickSceneInstanceIndex { index }),
                parse_index(gradient_index)?,
                parse_rgb(msg)?,
            ))
        }
        ["scene", "quick", index, input_kind, mode]
            if matches!(
                *input_kind,
                "activation_input" | "flash_input" | "dimmer_input"
            ) =>
        {
            let index = parse_quick_index(index)?;
            parse_target_input_action(
                msg,
                SceneInstanceUnion::Quick(QuickSceneInstanceIndex { index }),
                input_kind,
                mode,
            )
        }
        ["scene", "quick", index, curve_field, sub_action]
            if matches!(*curve_field, "opacity" | "beat_offset") =>
        {
            let index = parse_quick_index(index)?;
            parse_target_curve_action(
                msg,
                SceneInstanceUnion::Quick(QuickSceneInstanceIndex { index }),
                curve_field,
                sub_action,
            )
        }
        ["scene", "quick", index, action] => {
            let index = parse_quick_index(index)?;
            let target = SceneInstanceUnion::Quick(QuickSceneInstanceIndex { index });
            parse_target_action(msg, target, action)
        }
        ["scene", "reorder", from_col, from_row, to_col, to_row] => {
            let from = parse_grid_location(from_col, from_row)?;
            let to = parse_grid_location(to_col, to_row)?;
            Ok(UiAction::SwapScenes(from, to))
        }

        _ => {
            warn!("Received unknown osc message: {}", msg.addr);
            Err(())
        }
    }
}

fn parse_target_effect_action(
    msg: &OscMessage,
    target: SceneInstanceUnion,
    effect_index: usize,
    action: &str,
) -> Result<UiAction, ()> {
    match action {
        "opacity" => Ok(UiAction::SetSceneEffectOpacity(
            target,
            effect_index,
            parse_f32(msg, 0)?,
        )),
        "color_shift" => Ok(UiAction::SetSceneEffectColorShift(
            target,
            effect_index,
            parse_f32(msg, 0)?,
        )),
        "beat_progression" => Ok(UiAction::SetSceneEffectBeatProgression(
            target,
            effect_index,
            parse_f32(msg, 0)?,
        )),
        "beat_offset" => Ok(UiAction::SetSceneEffectBeatOffset(
            target,
            effect_index,
            parse_f32(msg, 0)?,
        )),
        "speed_exponent" => Ok(UiAction::SetSceneEffectSpeedExponent(
            target,
            effect_index,
            parse_i32(msg, 0)?,
        )),
        "group_index" => Ok(UiAction::SetSceneEffectGroupIndex(
            target,
            effect_index,
            parse_usize(msg, 0)?,
        )),
        "animation" => Ok(UiAction::SetSceneEffectAnimation(
            target,
            effect_index,
            parse_optional_animation_id(msg, 0)?,
        )),
        "delete" => Ok(UiAction::RemoveSceneEffect(target, effect_index)),
        "clone" => Ok(UiAction::CloneSceneEffect(target, effect_index)),
        _ => Err(()),
    }
}

fn parse_project_input_clear(event_kind: &str) -> Result<UiAction, ()> {
    match event_kind {
        "tap" => Ok(UiAction::ClearProjectTapInput),
        "blackout" => Ok(UiAction::ClearProjectBlackoutInput),
        "blackout_hold" => Ok(UiAction::ClearProjectBlackoutHoldInput),
        "half" => Ok(UiAction::ClearProjectHalfInput),
        "double" => Ok(UiAction::ClearProjectDoubleInput),
        _ => Err(()),
    }
}

fn parse_project_input_add_artnet(event_kind: &str, channel: u16) -> Result<UiAction, ()> {
    match event_kind {
        "tap" => Ok(UiAction::AddProjectTapInputArtnet(channel)),
        "blackout" => Ok(UiAction::AddProjectBlackoutInputArtnet(channel)),
        "blackout_hold" => Ok(UiAction::AddProjectBlackoutHoldInputArtnet(channel)),
        "half" => Ok(UiAction::AddProjectHalfInputArtnet(channel)),
        "double" => Ok(UiAction::AddProjectDoubleInputArtnet(channel)),
        _ => Err(()),
    }
}

fn parse_project_input_remove_artnet(event_kind: &str, channel: u16) -> Result<UiAction, ()> {
    match event_kind {
        "tap" => Ok(UiAction::RemoveProjectTapInputArtnet(channel)),
        "blackout" => Ok(UiAction::RemoveProjectBlackoutInputArtnet(channel)),
        "blackout_hold" => Ok(UiAction::RemoveProjectBlackoutHoldInputArtnet(channel)),
        "half" => Ok(UiAction::RemoveProjectHalfInputArtnet(channel)),
        "double" => Ok(UiAction::RemoveProjectDoubleInputArtnet(channel)),
        _ => Err(()),
    }
}

fn parse_target_input_action(
    msg: &OscMessage,
    target: SceneInstanceUnion,
    input_kind: &str,
    mode: &str,
) -> Result<UiAction, ()> {
    let input = match mode {
        "clear" => None,
        "artnet" => Some(InputEvent::Artnet(parse_u16(msg, 0)?)),
        _ => return Err(()),
    };

    match input_kind {
        "activation_input" => Ok(UiAction::SetSceneActivationInput(target, input)),
        "flash_input" => Ok(UiAction::SetSceneFlashInput(target, input)),
        "dimmer_input" => Ok(UiAction::SetSceneDimmerInput(target, input)),
        _ => Err(()),
    }
}

fn parse_target_action(
    msg: &OscMessage,
    target: SceneInstanceUnion,
    action: &str,
) -> Result<UiAction, ()> {
    match action {
        "opacity" => Ok(UiAction::SetSceneOpacity(target, parse_f32(msg, 0)?)),
        "active" => Ok(UiAction::SetSceneActive(target, parse_bool(msg, 0)?)),
        "toggle" => Ok(UiAction::ToggleSceneActive(target)),
        "select" => Ok(UiAction::SelectScene(target)),
        "name" => Ok(UiAction::SetSceneName(target, parse_string(msg, 0)?)),
        "color" => Ok(UiAction::SetSceneColor(target, parse_scene_color(msg, 0)?)),
        "input_dimmer" => Ok(UiAction::SetSceneInputDimmer(target, parse_f32(msg, 0)?)),
        "ignore_main_dimmer" => Ok(UiAction::SetSceneIgnoreMainDimmer(
            target,
            parse_bool(msg, 0)?,
        )),
        "beat_offset" => Ok(UiAction::SetSceneBeatOffset(target, parse_f32(msg, 0)?)),
        "set_offset_on_flash" => Ok(UiAction::SetSceneSetOffsetOnFlash(
            target,
            parse_bool(msg, 0)?,
        )),
        "palette_overwrite" => Ok(UiAction::SetScenePaletteOverwriteFromAsset(
            target,
            parse_scene_palette_overwrite(msg, 0)?,
        )),
        "delete" => Ok(UiAction::DeleteSceneInstancePath(target)),
        "clone" => Ok(UiAction::CloneSceneInstancePath(target)),
        _ => Err(()),
    }
}

fn parse_target_palette_overwrite_action(
    msg: &OscMessage,
    target: SceneInstanceUnion,
    action: &str,
) -> Result<UiAction, ()> {
    match action {
        "primary" => Ok(UiAction::SetScenePaletteOverwritePrimary(
            target,
            parse_rgb(msg)?,
        )),
        "secondary" => Ok(UiAction::SetScenePaletteOverwriteSecondary(
            target,
            parse_rgb(msg)?,
        )),
        _ => Err(()),
    }
}

fn parse_scene_palette_overwrite(
    msg: &OscMessage,
    index: usize,
) -> Result<Option<Option<AssetId<Palette>>>, ()> {
    let raw = parse_string(msg, index)?;
    if raw.eq_ignore_ascii_case("inherit") {
        return Ok(None);
    }
    if raw.eq_ignore_ascii_case("none") {
        return Ok(Some(None));
    }
    let uuid = Uuid::parse_str(&raw).map_err(|_| ())?;
    Ok(Some(Some(AssetId::from_uuid(uuid))))
}

fn parse_grid_location(col: &str, row: &str) -> Result<GridLocation, ()> {
    let col = col.parse::<usize>().map_err(|_| ())?;
    let row = row.parse::<usize>().map_err(|_| ())?;
    Ok(GridLocation::new(col, row))
}

fn parse_quick_index(index: &str) -> Result<usize, ()> {
    let index = index.parse::<usize>().map_err(|_| ())?;
    Ok(index)
}

fn parse_scene_id(msg: &OscMessage, index: usize) -> Result<AssetId<Scene>, ()> {
    Ok(AssetId::from_uuid(parse_uuid(msg, index)?))
}

fn parse_optional_animation_id(
    msg: &OscMessage,
    index: usize,
) -> Result<Option<AssetId<Animation>>, ()> {
    parse_optional_uuid(msg, index).map(|id| id.map(AssetId::from_uuid))
}

fn parse_optional_palette_id(
    msg: &OscMessage,
    index: usize,
) -> Result<Option<AssetId<Palette>>, ()> {
    parse_optional_uuid(msg, index).map(|id| id.map(AssetId::from_uuid))
}

fn parse_animation_id_from_path(animation_id: &str) -> Result<AssetId<Animation>, ()> {
    let uuid = Uuid::parse_str(animation_id).map_err(|_| ())?;
    Ok(AssetId::from_uuid(uuid))
}

fn parse_argument_kind(msg: &OscMessage, index: usize) -> Result<ArgumentKindId, ()> {
    let value = parse_string(msg, index)?;
    match value.to_lowercase().as_str() {
        "center" => Ok(ArgumentKindId::Center),
        "selection" => Ok(ArgumentKindId::Selection),
        "slider" => Ok(ArgumentKindId::Slider),
        "checkbox" => Ok(ArgumentKindId::Checkbox),
        "percentage" => Ok(ArgumentKindId::Percentage),
        "degrees" => Ok(ArgumentKindId::Degrees),
        _ => Err(()),
    }
}

fn parse_uuid(msg: &OscMessage, index: usize) -> Result<Uuid, ()> {
    let raw = parse_string(msg, index)?;
    Uuid::parse_str(&raw).map_err(|_| ())
}

fn parse_optional_uuid(msg: &OscMessage, index: usize) -> Result<Option<Uuid>, ()> {
    let raw = parse_string(msg, index)?;
    if raw.eq_ignore_ascii_case("none") {
        return Ok(None);
    }
    Uuid::parse_str(&raw).map(Some).map_err(|_| ())
}

fn parse_index(value: &str) -> Result<usize, ()> {
    value.parse::<usize>().map_err(|_| ())
}

fn parse_rgb(msg: &OscMessage) -> Result<[f32; 3], ()> {
    Ok([parse_f32(msg, 0)?, parse_f32(msg, 1)?, parse_f32(msg, 2)?])
}

fn arg_at(msg: &OscMessage, index: usize) -> Result<&OscType, ()> {
    msg.args.get(index).ok_or(())
}

fn parse_f32(msg: &OscMessage, index: usize) -> Result<f32, ()> {
    match arg_at(msg, index)? {
        OscType::Float(value) => Ok(*value),
        OscType::Double(value) => Ok(*value as f32),
        OscType::Int(value) => Ok(*value as f32),
        OscType::Long(value) => Ok(*value as f32),
        _ => Err(()),
    }
}

fn parse_u16(msg: &OscMessage, index: usize) -> Result<u16, ()> {
    match arg_at(msg, index)? {
        OscType::Int(value) => u16::try_from(*value).map_err(|_| ()),
        OscType::Long(value) => u16::try_from(*value).map_err(|_| ()),
        _ => Err(()),
    }
}

fn parse_u32(msg: &OscMessage, index: usize) -> Result<u32, ()> {
    match arg_at(msg, index)? {
        OscType::Int(value) => u32::try_from(*value).map_err(|_| ()),
        OscType::Long(value) => u32::try_from(*value).map_err(|_| ()),
        _ => Err(()),
    }
}

fn parse_i32(msg: &OscMessage, index: usize) -> Result<i32, ()> {
    match arg_at(msg, index)? {
        OscType::Int(value) => Ok(*value),
        OscType::Long(value) => i32::try_from(*value).map_err(|_| ()),
        _ => Err(()),
    }
}

fn parse_usize(msg: &OscMessage, index: usize) -> Result<usize, ()> {
    match arg_at(msg, index)? {
        OscType::Int(value) => usize::try_from(*value).map_err(|_| ()),
        OscType::Long(value) => usize::try_from(*value).map_err(|_| ()),
        _ => Err(()),
    }
}

fn parse_bool(msg: &OscMessage, index: usize) -> Result<bool, ()> {
    match arg_at(msg, index)? {
        OscType::Bool(value) => Ok(*value),
        OscType::Int(value) => match *value {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(()),
        },
        OscType::Long(value) => match *value {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(()),
        },
        _ => Err(()),
    }
}

fn parse_string(msg: &OscMessage, index: usize) -> Result<String, ()> {
    match arg_at(msg, index)? {
        OscType::String(value) => Ok(value.clone()),
        _ => Err(()),
    }
}

fn parse_curve_id(msg: &OscMessage, index: usize) -> Result<Option<AssetId<Curve>>, ()> {
    let raw = parse_string(msg, index)?;
    if raw.eq_ignore_ascii_case("none") {
        return Ok(None);
    }
    let uuid = Uuid::parse_str(&raw).map_err(|_| ())?;
    Ok(Some(AssetId::from_uuid(uuid)))
}

fn parse_target_curve_action(
    msg: &OscMessage,
    target: SceneInstanceUnion,
    field: &str,
    sub_action: &str,
) -> Result<UiAction, ()> {
    match (field, sub_action) {
        ("opacity", "value") => Ok(UiAction::SetSceneOpacityMultiplier(target, parse_f32(msg, 0)?)),
        ("opacity", "static") => Ok(UiAction::SetSceneOpacity(target, parse_f32(msg, 0)?)),
        ("opacity", "curve") => Ok(UiAction::SetSceneOpacityCurve(target, parse_curve_id(msg, 0)?)),
        ("beat_offset", "value") => Ok(UiAction::SetSceneBeatOffsetMultiplier(target, parse_f32(msg, 0)?)),
        ("beat_offset", "static") => Ok(UiAction::SetSceneBeatOffset(target, parse_f32(msg, 0)?)),
        ("beat_offset", "curve") => Ok(UiAction::SetSceneBeatOffsetCurve(target, parse_curve_id(msg, 0)?)),
        _ => Err(()),
    }
}

fn parse_target_effect_curve_action(
    msg: &OscMessage,
    target: SceneInstanceUnion,
    effect_index: usize,
    field: &str,
    sub_action: &str,
) -> Result<UiAction, ()> {
    match (field, sub_action) {
        ("opacity", "value") => Ok(UiAction::SetSceneEffectOpacityMultiplier(target, effect_index, parse_f32(msg, 0)?)),
        ("opacity", "static") => Ok(UiAction::SetSceneEffectOpacity(target, effect_index, parse_f32(msg, 0)?)),
        ("opacity", "curve") => Ok(UiAction::SetSceneEffectOpacityCurve(target, effect_index, parse_curve_id(msg, 0)?)),
        ("color_shift", "value") => Ok(UiAction::SetSceneEffectColorShiftMultiplier(target, effect_index, parse_f32(msg, 0)?)),
        ("color_shift", "static") => Ok(UiAction::SetSceneEffectColorShift(target, effect_index, parse_f32(msg, 0)?)),
        ("color_shift", "curve") => Ok(UiAction::SetSceneEffectColorShiftCurve(target, effect_index, parse_curve_id(msg, 0)?)),
        ("beat_progression", "value") => Ok(UiAction::SetSceneEffectBeatProgressionMultiplier(target, effect_index, parse_f32(msg, 0)?)),
        ("beat_progression", "static") => Ok(UiAction::SetSceneEffectBeatProgression(target, effect_index, parse_f32(msg, 0)?)),
        ("beat_progression", "curve") => Ok(UiAction::SetSceneEffectBeatProgressionCurve(target, effect_index, parse_curve_id(msg, 0)?)),
        ("beat_offset", "value") => Ok(UiAction::SetSceneEffectBeatOffsetMultiplier(target, effect_index, parse_f32(msg, 0)?)),
        ("beat_offset", "static") => Ok(UiAction::SetSceneEffectBeatOffset(target, effect_index, parse_f32(msg, 0)?)),
        ("beat_offset", "curve") => Ok(UiAction::SetSceneEffectBeatOffsetCurve(target, effect_index, parse_curve_id(msg, 0)?)),
        _ => Err(()),
    }
}

fn parse_scene_color(msg: &OscMessage, index: usize) -> Result<SceneInstanceColor, ()> {
    let value = parse_string(msg, index)?;
    match value.to_lowercase().as_str() {
        "red" => Ok(SceneInstanceColor::Red),
        "green" => Ok(SceneInstanceColor::Green),
        "blue" => Ok(SceneInstanceColor::Blue),
        "white" => Ok(SceneInstanceColor::White),
        "orange" => Ok(SceneInstanceColor::Orange),
        "yellow" => Ok(SceneInstanceColor::Yellow),
        "purple" => Ok(SceneInstanceColor::Purple),
        "pink" => Ok(SceneInstanceColor::Pink),
        "black" => Ok(SceneInstanceColor::Black),
        _ => Err(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(addr: &str, args: Vec<OscType>) -> OscMessage {
        OscMessage {
            addr: addr.to_string(),
            args,
        }
    }

    #[test]
    fn parses_project_main_dimmer() {
        let msg = msg("/project/main_dimmer", vec![OscType::Float(0.5)]);
        let action = parse_message(&msg).expect("main dimmer should parse");
        match action {
            UiAction::SetMainDimmer(value) => assert_eq!(value, 0.5),
            _ => panic!("wrong action variant"),
        }
    }

    #[test]
    fn parses_scene_grid_opacity() {
        let msg = msg("/scene/grid/2/1/opacity", vec![OscType::Float(0.7)]);
        let action = parse_message(&msg).expect("grid opacity should parse");
        match action {
            UiAction::SetSceneOpacity(SceneInstanceUnion::Grid(location), value) => {
                assert_eq!(location.col, 2);
                assert_eq!(location.row, 1);
                assert_eq!(value, 0.7);
            }
            _ => panic!("wrong action variant"),
        }
    }

    #[test]
    fn parses_scene_quick_active() {
        let msg = msg("/scene/quick/3/active", vec![OscType::Int(1)]);
        let action = parse_message(&msg).expect("quick active should parse");
        match action {
            UiAction::SetSceneActive(SceneInstanceUnion::Quick(index), value) => {
                assert_eq!(index.index, 3);
                assert!(value);
            }
            _ => panic!("wrong action variant"),
        }
    }

    #[test]
    fn parses_selected_scene_toggle() {
        let msg = msg("/scene/selected/toggle", vec![]);
        let action = parse_message(&msg).expect("selected toggle should parse");
        assert!(matches!(
            action,
            UiAction::ToggleSceneActive(SceneInstanceUnion::Selected)
        ));
    }

    #[test]
    fn parses_large_grid_location_without_static_bounds_check() {
        let msg = msg("/scene/grid/999/777/opacity", vec![OscType::Float(0.2)]);
        match parse_message(&msg).expect("large grid location should parse") {
            UiAction::SetSceneOpacity(SceneInstanceUnion::Grid(location), value) => {
                assert_eq!(location.col, 999);
                assert_eq!(location.row, 777);
                assert_eq!(value, 0.2);
            }
            _ => panic!("wrong action variant"),
        }
    }

    #[test]
    fn parses_project_palette_none() {
        let msg = msg(
            "/project/palette",
            vec![OscType::String("none".to_string())],
        );
        let action = parse_message(&msg).expect("palette none should parse");
        match action {
            UiAction::SetProjectPaletteFromAsset(value) => assert!(value.is_none()),
            _ => panic!("wrong action variant"),
        }
    }

    #[test]
    fn parses_scene_effect_config_f32() {
        let msg = msg(
            "/scene/grid/1/2/effect/0/config/f32/3",
            vec![OscType::Float(0.25)],
        );
        let action = parse_message(&msg).expect("effect config should parse");
        match action {
            UiAction::SetSceneEffectAnimationConfigF32(
                SceneInstanceUnion::Grid(location),
                effect_index,
                config_index,
                value,
            ) => {
                assert_eq!(location.col, 1);
                assert_eq!(location.row, 2);
                assert_eq!(effect_index, 0);
                assert_eq!(config_index, 3);
                assert_eq!(value, 0.25);
            }
            _ => panic!("wrong action variant"),
        }
    }

    #[test]
    fn parses_selected_scene_effect_speed_exponent() {
        let msg = msg(
            "/scene/selected/effect/2/speed_exponent",
            vec![OscType::Int(-1)],
        );
        let action = parse_message(&msg).expect("selected effect speed should parse");
        match action {
            UiAction::SetSceneEffectSpeedExponent(
                SceneInstanceUnion::Selected,
                effect_index,
                value,
            ) => {
                assert_eq!(effect_index, 2);
                assert_eq!(value, -1);
            }
            _ => panic!("wrong action variant"),
        }
    }

    #[test]
    fn parses_project_palette_gradient_edit() {
        let msg = msg(
            "/project/palette/gradient/4",
            vec![
                OscType::Float(0.1),
                OscType::Float(0.2),
                OscType::Float(0.3),
            ],
        );
        let action = parse_message(&msg).expect("palette gradient should parse");
        match action {
            UiAction::SetProjectPaletteGradient(index, rgb) => {
                assert_eq!(index, 4);
                assert_eq!(rgb, [0.1, 0.2, 0.3]);
            }
            _ => panic!("wrong action variant"),
        }
    }

    #[test]
    fn parses_scene_effect_add() {
        let msg = msg("/scene/grid/3/1/effect/add", vec![]);
        let action = parse_message(&msg).expect("effect add should parse");
        match action {
            UiAction::AddSceneEffect(SceneInstanceUnion::Grid(location)) => {
                assert_eq!(location.col, 3);
                assert_eq!(location.row, 1);
            }
            _ => panic!("wrong action variant"),
        }
    }

    #[test]
    fn parses_selected_scene_effect_delete() {
        let msg = msg("/scene/selected/effect/5/delete", vec![]);
        let action = parse_message(&msg).expect("effect delete should parse");
        match action {
            UiAction::RemoveSceneEffect(SceneInstanceUnion::Selected, index) => {
                assert_eq!(index, 5)
            }
            _ => panic!("wrong action variant"),
        }
    }

    #[test]
    fn parses_grid_activation_input_artnet() {
        let msg = msg(
            "/scene/grid/0/0/activation_input/artnet",
            vec![OscType::Int(77)],
        );
        let action = parse_message(&msg).expect("activation input should parse");
        match action {
            UiAction::SetSceneActivationInput(
                SceneInstanceUnion::Grid(location),
                Some(InputEvent::Artnet(channel)),
            ) => {
                assert_eq!(location.col, 0);
                assert_eq!(location.row, 0);
                assert_eq!(channel, 77);
            }
            _ => panic!("wrong action variant"),
        }
    }

    #[test]
    fn parses_selected_palette_overwrite_none() {
        let msg = msg(
            "/scene/selected/palette_overwrite",
            vec![OscType::String("none".to_string())],
        );
        let action = parse_message(&msg).expect("palette overwrite should parse");
        match action {
            UiAction::SetScenePaletteOverwriteFromAsset(SceneInstanceUnion::Selected, value) => {
                assert!(matches!(value, Some(None)));
            }
            _ => panic!("wrong action variant"),
        }
    }

    #[test]
    fn parses_selected_palette_overwrite_primary_edit() {
        let msg = msg(
            "/scene/selected/palette_overwrite/primary",
            vec![
                OscType::Float(0.2),
                OscType::Float(0.4),
                OscType::Float(0.6),
            ],
        );
        let action = parse_message(&msg).expect("selected palette primary should parse");
        match action {
            UiAction::SetScenePaletteOverwritePrimary(SceneInstanceUnion::Selected, rgb) => {
                assert_eq!(rgb, [0.2, 0.4, 0.6]);
            }
            _ => panic!("wrong action variant"),
        }
    }

    #[test]
    fn parses_grid_palette_overwrite_gradient_edit() {
        let msg = msg(
            "/scene/grid/4/2/palette_overwrite/gradient/3",
            vec![
                OscType::Float(0.3),
                OscType::Float(0.5),
                OscType::Float(0.7),
            ],
        );
        let action = parse_message(&msg).expect("grid palette gradient should parse");
        match action {
            UiAction::SetScenePaletteOverwriteGradient(
                SceneInstanceUnion::Grid(location),
                index,
                rgb,
            ) => {
                assert_eq!(location.col, 4);
                assert_eq!(location.row, 2);
                assert_eq!(index, 3);
                assert_eq!(rgb, [0.3, 0.5, 0.7]);
            }
            _ => panic!("wrong action variant"),
        }
    }

    #[test]
    fn parses_project_artnet_control_universe() {
        let msg = msg("/project/artnet_control/universe", vec![OscType::Int(1337)]);
        let action = parse_message(&msg).expect("artnet control universe should parse");
        match action {
            UiAction::SetProjectArtnetControlUniverse(universe) => {
                assert_eq!(universe, 1337);
            }
            _ => panic!("wrong action variant"),
        }
    }

    #[test]
    fn parses_selected_groups_overwrite_set() {
        let msg = msg(
            "/scene/selected/groups_overwrite/2",
            vec![OscType::String("front".to_string())],
        );
        let action = parse_message(&msg).expect("groups overwrite set should parse");
        match action {
            UiAction::SetSceneGroupsOverwriteEntry(SceneInstanceUnion::Selected, index, group_name) => {
                assert_eq!(index, 2);
                assert_eq!(group_name, "front");
            }
            _ => panic!("wrong action variant"),
        }
    }

    #[test]
    fn parses_animation_argument_kind_edit() {
        let animation_id = "11111111-1111-1111-1111-111111111111";
        let msg = msg(
            &format!("/asset/animation/{animation_id}/argument/1/kind"),
            vec![OscType::String("degrees".to_string())],
        );
        let action = parse_message(&msg).expect("animation argument kind should parse");
        match action {
            UiAction::SetAnimationArgumentKind(_, argument_index, kind) => {
                assert_eq!(argument_index, 1);
                assert_eq!(kind, ArgumentKindId::Degrees);
            }
            _ => panic!("wrong action variant"),
        }
    }

    #[test]
    fn parses_project_group_set() {
        let msg = msg(
            "/project/groups/3",
            vec![OscType::String("wash".to_string())],
        );
        let action = parse_message(&msg).expect("project group set should parse");
        match action {
            UiAction::SetProjectGroup(index, group_name) => {
                assert_eq!(index, 3);
                assert_eq!(group_name, "wash");
            }
            _ => panic!("wrong action variant"),
        }
    }

    #[test]
    fn parses_project_input_add_artnet() {
        let msg = msg("/project/input/tap/add/artnet", vec![OscType::Int(12)]);
        let action = parse_message(&msg).expect("project input add should parse");
        match action {
            UiAction::AddProjectTapInputArtnet(channel) => assert_eq!(channel, 12),
            _ => panic!("wrong action variant"),
        }
    }
}
