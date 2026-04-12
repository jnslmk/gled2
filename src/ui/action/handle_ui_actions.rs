use super::*;

impl App {
    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn handle_ui_actions(&mut self) {
        loop {
            let Ok(Some(action)) = self.ui_action_receiver.try_recv() else {
                return;
            };
            log::trace!("Handling ui action: {action:?}");

            if let Some(project) = &mut self.project
                && apply_palette_action(
                    project,
                    self.selected_scene_instance,
                    &self.collections,
                    &action,
                )
            {
                continue;
            }

            match (&mut self.project, action) {
                (Some(project), UiAction::AddScene(pos, scene)) => {
                    project.add_scene(pos, scene, &self.collections);
                }
                (Some(project), UiAction::DeleteSceneInstance { location }) => {
                    project.remove_scene_instance(location);
                }
                (Some(project), UiAction::DeleteSelectedSceneInstance) => {
                    project.remove_scene_instance(self.selected_scene_instance);
                }

                (Some(project), UiAction::CloneSelectedSceneInstance) => {
                    if let Some(scene_id) = project
                        .get_scenes_instance(&self.selected_scene_instance)
                        .map(|scene_instance| scene_instance.scene_id)
                    {
                        project.add_scene(
                            project.next_empty_grid_location(self.selected_scene_instance),
                            scene_id,
                            &self.collections,
                        );
                    }
                }
                (Some(project), UiAction::CloneSceneInstance(location)) => {
                    if let Some(scene_id) = project
                        .get_scenes_instance(&location)
                        .map(|scene_instance| scene_instance.scene_id)
                    {
                        project.add_scene(
                            project.next_empty_grid_location(location),
                            scene_id,
                            &self.collections,
                        );
                    }
                }
                (Some(project), UiAction::ReloadShaderCode(animation)) => {
                    project.reload_shader_code(animation, &self.collections);
                    self.windows
                        .scenes
                        .reload_shader_code(animation, &mut self.collections);
                }
                (Some(project), UiAction::SendPositions) => {
                    project.send_positions();
                }
                (Some(project), UiAction::SetSvg(svg)) => {
                    project.svg = svg;
                    project.remove_nonexistant_groups();
                }
                (Some(project), UiAction::SetSceneOpacity(path, opacity)) => {
                    if let Some(scene_instance) =
                        project.scene_instance_by_location_or_quick_index(path)
                    {
                        scene_instance.opacity = MultipliedCurve::new_multiplier(opacity);
                    }
                }
                (Some(project), UiAction::SetSelectedSceneOpacity(opacity)) => {
                    if let Some(scene_instance) =
                        project.get_scenes_instance(&self.selected_scene_instance)
                    {
                        scene_instance.opacity = MultipliedCurve::new_multiplier(opacity);
                    }
                }
                (Some(project), UiAction::SetMainDimmer(dimmer)) => {
                    project.main_dimmer = dimmer;
                }
                (Some(project), UiAction::SetProjectAutoModeActive(active)) => {
                    project.auto_mode_active = active;
                }
                (Some(project), UiAction::SetProjectAutoModeSeconds(seconds)) => {
                    project.auto_mode_seconds = seconds;
                }
                (Some(project), UiAction::SetProjectAutoModeMaxScenes(max_scenes)) => {
                    project.auto_mode_max_scenes = max_scenes;
                }
                (Some(project), UiAction::SetProjectArtnetControlActive(active)) => {
                    project.artnet_control_config(|config| config.active = active);
                }
                (Some(project), UiAction::SetProjectArtnetControlUniverse(universe)) => {
                    project.artnet_control_config(|config| config.universe = universe);
                }
                (Some(project), UiAction::SetProjectGroup(group_index, group_name)) => {
                    project.groups.insert(group_index, Group(group_name));
                }
                (Some(project), UiAction::RemoveProjectGroup(group_index)) => {
                    project.groups.remove(group_index);
                }
                (Some(project), UiAction::ClearProjectGroups) => {
                    project.groups.clear();
                }
                (Some(project), UiAction::AddProjectTapInputArtnet(channel)) => {
                    project.tap_input_events.insert(InputEvent::Artnet(channel));
                }
                (Some(project), UiAction::RemoveProjectTapInputArtnet(channel)) => {
                    project
                        .tap_input_events
                        .remove(&InputEvent::Artnet(channel));
                }
                (Some(project), UiAction::ClearProjectTapInput) => {
                    project.tap_input_events.clear();
                }
                (Some(project), UiAction::AddProjectBlackoutInputArtnet(channel)) => {
                    project
                        .blackout_input_events
                        .insert(InputEvent::Artnet(channel));
                }
                (Some(project), UiAction::RemoveProjectBlackoutInputArtnet(channel)) => {
                    project
                        .blackout_input_events
                        .remove(&InputEvent::Artnet(channel));
                }
                (Some(project), UiAction::ClearProjectBlackoutInput) => {
                    project.blackout_input_events.clear();
                }
                (Some(project), UiAction::AddProjectBlackoutHoldInputArtnet(channel)) => {
                    project
                        .blackout_hold_input_events
                        .insert(InputEvent::Artnet(channel));
                }
                (Some(project), UiAction::RemoveProjectBlackoutHoldInputArtnet(channel)) => {
                    project
                        .blackout_hold_input_events
                        .remove(&InputEvent::Artnet(channel));
                }
                (Some(project), UiAction::ClearProjectBlackoutHoldInput) => {
                    project.blackout_hold_input_events.clear();
                }
                (Some(project), UiAction::AddProjectHalfInputArtnet(channel)) => {
                    project
                        .half_input_events
                        .insert(InputEvent::Artnet(channel));
                }
                (Some(project), UiAction::RemoveProjectHalfInputArtnet(channel)) => {
                    project
                        .half_input_events
                        .remove(&InputEvent::Artnet(channel));
                }
                (Some(project), UiAction::ClearProjectHalfInput) => {
                    project.half_input_events.clear();
                }
                (Some(project), UiAction::AddProjectDoubleInputArtnet(channel)) => {
                    project
                        .double_input_events
                        .insert(InputEvent::Artnet(channel));
                }
                (Some(project), UiAction::RemoveProjectDoubleInputArtnet(channel)) => {
                    project
                        .double_input_events
                        .remove(&InputEvent::Artnet(channel));
                }
                (Some(project), UiAction::ClearProjectDoubleInput) => {
                    project.double_input_events.clear();
                }
                (Some(project), UiAction::ToggleSceneActive(location)) => {
                    if let Some(scene_instance) =
                        project.scene_instance_by_location_or_quick_index(location)
                    {
                        scene_instance.active = !scene_instance.active;
                    }
                }
                (Some(project), UiAction::SetSceneActive(location, active)) => {
                    if let Some(scene_instance) =
                        project.scene_instance_by_location_or_quick_index(location)
                    {
                        scene_instance.active = active;
                    }
                }
                (Some(project), UiAction::SetSelectedSceneActive(active)) => {
                    if let Some(scene_instance) =
                        project.get_scenes_instance(&self.selected_scene_instance)
                    {
                        scene_instance.active = active;
                    }
                }
                (Some(project), UiAction::ToggleSelectedSceneActive) => {
                    if let Some(scene_instance) =
                        project.get_scenes_instance(&self.selected_scene_instance)
                    {
                        scene_instance.active = !scene_instance.active;
                    }
                }
                (Some(project), UiAction::SelectScene(location)) => {
                    if let Some(pos) = project.location_by_location_or_quick_index(location) {
                        self.set_selected_scene_instance(pos);
                    }
                }
                (Some(project), UiAction::SelectSceneByLocation(location)) => {
                    if project.scenes_instances_grid.contains_key(&location) {
                        self.set_selected_scene_instance(location);
                    }
                }
                (Some(project), UiAction::SetAudioDevice(device_id)) => {
                    project.audio_input_device = device_id.clone();
                    self.audio_pool.selected_device = device_id;
                    self.audio_pool.restart_fft();
                }
                (Some(project), UiAction::SwapScenes(from, to)) => {
                    let to_item = project.remove_scene_instance(to);
                    let from_item = project.remove_scene_instance(from);
                    if let Some(from_item) = from_item {
                        project.add_scene_instance(to, from_item);
                    }
                    if let Some(to_item) = to_item {
                        project.add_scene_instance(from, to_item);
                    }
                    self.set_selected_scene_instance(to);
                }
                (Some(project), UiAction::SetSceneName(path, name)) => {
                    if let Some(scene_instance) =
                        project.scene_instance_by_location_or_quick_index(path)
                    {
                        scene_instance.name = name;
                    }
                }
                (Some(project), UiAction::SetSelectedSceneName(name)) => {
                    if let Some(scene_instance) =
                        project.get_scenes_instance(&self.selected_scene_instance)
                    {
                        scene_instance.name = name;
                    }
                }
                (Some(project), UiAction::SetSceneColor(path, color)) => {
                    if let Some(scene_instance) =
                        project.scene_instance_by_location_or_quick_index(path)
                    {
                        scene_instance.color = color;
                    }
                }
                (Some(project), UiAction::SetSelectedSceneColor(color)) => {
                    if let Some(scene_instance) =
                        project.get_scenes_instance(&self.selected_scene_instance)
                    {
                        scene_instance.color = color;
                    }
                }
                (Some(project), UiAction::SetSceneInputDimmer(path, dimmer)) => {
                    if let Some(scene_instance) =
                        project.scene_instance_by_location_or_quick_index(path)
                    {
                        scene_instance.input_dimmer = dimmer;
                    }
                }
                (Some(project), UiAction::SetSelectedSceneInputDimmer(dimmer)) => {
                    if let Some(scene_instance) =
                        project.get_scenes_instance(&self.selected_scene_instance)
                    {
                        scene_instance.input_dimmer = dimmer;
                    }
                }
                (Some(project), UiAction::SetSceneIgnoreMainDimmer(path, ignore)) => {
                    if let Some(scene_instance) =
                        project.scene_instance_by_location_or_quick_index(path)
                    {
                        scene_instance.ignore_main_dimmer = ignore;
                    }
                }
                (Some(project), UiAction::SetSelectedSceneIgnoreMainDimmer(ignore)) => {
                    if let Some(scene_instance) =
                        project.get_scenes_instance(&self.selected_scene_instance)
                    {
                        scene_instance.ignore_main_dimmer = ignore;
                    }
                }
                (Some(project), UiAction::SetSceneBeatOffset(path, beat_offset)) => {
                    if let Some(scene_instance) =
                        project.scene_instance_by_location_or_quick_index(path)
                    {
                        scene_instance.beat_progression_offset =
                            MultipliedCurve::new_multiplier(beat_offset);
                    }
                }
                (Some(project), UiAction::SetSelectedSceneBeatOffset(beat_offset)) => {
                    if let Some(scene_instance) =
                        project.get_scenes_instance(&self.selected_scene_instance)
                    {
                        scene_instance.beat_progression_offset =
                            MultipliedCurve::new_multiplier(beat_offset);
                    }
                }
                (Some(project), UiAction::SetSceneSetOffsetOnFlash(path, set_offset_on_flash)) => {
                    if let Some(scene_instance) =
                        project.scene_instance_by_location_or_quick_index(path)
                    {
                        scene_instance.set_offset_on_flash = set_offset_on_flash;
                    }
                }
                (
                    Some(project),
                    UiAction::SetSelectedSceneSetOffsetOnFlash(set_offset_on_flash),
                ) => {
                    if let Some(scene_instance) =
                        project.get_scenes_instance(&self.selected_scene_instance)
                    {
                        scene_instance.set_offset_on_flash = set_offset_on_flash;
                    }
                }
                (Some(project), UiAction::SetSceneActivationInput(path, input_event)) => {
                    if let Some(scene_instance) =
                        project.scene_instance_by_location_or_quick_index(path)
                    {
                        scene_instance.activation_input = input_event;
                    }
                }
                (Some(project), UiAction::SetSelectedSceneActivationInput(input_event)) => {
                    if let Some(scene_instance) =
                        project.get_scenes_instance(&self.selected_scene_instance)
                    {
                        scene_instance.activation_input = input_event;
                    }
                }
                (Some(project), UiAction::SetSceneFlashInput(path, input_event)) => {
                    if let Some(scene_instance) =
                        project.scene_instance_by_location_or_quick_index(path)
                    {
                        scene_instance.flash_input = input_event;
                    }
                }
                (Some(project), UiAction::SetSelectedSceneFlashInput(input_event)) => {
                    if let Some(scene_instance) =
                        project.get_scenes_instance(&self.selected_scene_instance)
                    {
                        scene_instance.flash_input = input_event;
                    }
                }
                (Some(project), UiAction::SetSceneDimmerInput(path, input_event)) => {
                    if let Some(scene_instance) =
                        project.scene_instance_by_location_or_quick_index(path)
                    {
                        scene_instance.dimmer_input = input_event;
                    }
                }
                (Some(project), UiAction::SetSelectedSceneDimmerInput(input_event)) => {
                    if let Some(scene_instance) =
                        project.get_scenes_instance(&self.selected_scene_instance)
                    {
                        scene_instance.dimmer_input = input_event;
                    }
                }
                (Some(project), UiAction::ClearSceneGroupsOverwrite(path)) => {
                    if let Some(scene_instance) =
                        project.scene_instance_by_location_or_quick_index(path)
                    {
                        scene_instance.groups_overwrite = None;
                    }
                }
                (Some(project), UiAction::ClearSelectedSceneGroupsOverwrite) => {
                    if let Some(scene_instance) =
                        project.get_scenes_instance(&self.selected_scene_instance)
                    {
                        scene_instance.groups_overwrite = None;
                    }
                }
                (
                    Some(project),
                    UiAction::SetSceneGroupsOverwriteEntry(path, group_index, group_name),
                ) => {
                    if let Some(scene_instance) =
                        project.scene_instance_by_location_or_quick_index(path)
                    {
                        let groups = scene_instance.groups_overwrite.get_or_insert_default();
                        groups.insert(group_index, Group(group_name));
                    }
                }
                (
                    Some(project),
                    UiAction::SetSelectedSceneGroupsOverwriteEntry(group_index, group_name),
                ) => {
                    if let Some(scene_instance) =
                        project.get_scenes_instance(&self.selected_scene_instance)
                    {
                        let groups = scene_instance.groups_overwrite.get_or_insert_default();
                        groups.insert(group_index, Group(group_name));
                    }
                }
                (Some(project), UiAction::RemoveSceneGroupsOverwriteEntry(path, group_index)) => {
                    if let Some(scene_instance) =
                        project.scene_instance_by_location_or_quick_index(path)
                        && let Some(groups) = scene_instance.groups_overwrite.as_mut()
                    {
                        groups.remove(group_index);
                        if groups.is_empty() {
                            scene_instance.groups_overwrite = None;
                        }
                    }
                }
                (Some(project), UiAction::RemoveSelectedSceneGroupsOverwriteEntry(group_index)) => {
                    if let Some(scene_instance) =
                        project.get_scenes_instance(&self.selected_scene_instance)
                        && let Some(groups) = scene_instance.groups_overwrite.as_mut()
                    {
                        groups.remove(group_index);
                        if groups.is_empty() {
                            scene_instance.groups_overwrite = None;
                        }
                    }
                }
                (Some(project), UiAction::SetSceneEffectOpacity(target, effect_index, opacity)) => {
                    if let Some(effect) = scene_effect_by_target(project, target, effect_index) {
                        effect.opacity = MultipliedCurve::new_multiplier(opacity);
                    }
                }
                (Some(project), UiAction::SetSelectedSceneEffectOpacity(effect_index, opacity)) => {
                    if let Some(effect) =
                        selected_scene_effect(project, self.selected_scene_instance, effect_index)
                    {
                        effect.opacity = MultipliedCurve::new_multiplier(opacity);
                    }
                }
                (
                    Some(project),
                    UiAction::SetSceneEffectColorShift(target, effect_index, color_shift),
                ) => {
                    if let Some(effect) = scene_effect_by_target(project, target, effect_index) {
                        effect.color_shift = MultipliedCurve::new_multiplier(color_shift);
                    }
                }
                (
                    Some(project),
                    UiAction::SetSelectedSceneEffectColorShift(effect_index, color_shift),
                ) => {
                    if let Some(effect) =
                        selected_scene_effect(project, self.selected_scene_instance, effect_index)
                    {
                        effect.color_shift = MultipliedCurve::new_multiplier(color_shift);
                    }
                }
                (
                    Some(project),
                    UiAction::SetSceneEffectBeatProgression(target, effect_index, beat_progression),
                ) => {
                    if let Some(effect) = scene_effect_by_target(project, target, effect_index) {
                        effect.beat_progression = MultipliedCurve::new_multiplier(beat_progression);
                    }
                }
                (
                    Some(project),
                    UiAction::SetSelectedSceneEffectBeatProgression(effect_index, beat_progression),
                ) => {
                    if let Some(effect) =
                        selected_scene_effect(project, self.selected_scene_instance, effect_index)
                    {
                        effect.beat_progression = MultipliedCurve::new_multiplier(beat_progression);
                    }
                }
                (
                    Some(project),
                    UiAction::SetSceneEffectBeatOffset(target, effect_index, beat_offset),
                ) => {
                    if let Some(effect) = scene_effect_by_target(project, target, effect_index) {
                        effect.beat_progression_offset =
                            MultipliedCurve::new_multiplier(beat_offset);
                    }
                }
                (
                    Some(project),
                    UiAction::SetSelectedSceneEffectBeatOffset(effect_index, beat_offset),
                ) => {
                    if let Some(effect) =
                        selected_scene_effect(project, self.selected_scene_instance, effect_index)
                    {
                        effect.beat_progression_offset =
                            MultipliedCurve::new_multiplier(beat_offset);
                    }
                }
                (
                    Some(project),
                    UiAction::SetSceneEffectSpeedExponent(target, effect_index, speed_exponent),
                ) => {
                    if let Some(effect) = scene_effect_by_target(project, target, effect_index) {
                        effect.speed_exponent = speed_exponent;
                    }
                }
                (
                    Some(project),
                    UiAction::SetSelectedSceneEffectSpeedExponent(effect_index, speed_exponent),
                ) => {
                    if let Some(effect) =
                        selected_scene_effect(project, self.selected_scene_instance, effect_index)
                    {
                        effect.speed_exponent = speed_exponent;
                    }
                }
                (
                    Some(project),
                    UiAction::SetSceneEffectGroupIndex(target, effect_index, group_index),
                ) => {
                    if let Some(effect) = scene_effect_by_target(project, target, effect_index) {
                        effect.group_index = group_index;
                    }
                }
                (
                    Some(project),
                    UiAction::SetSelectedSceneEffectGroupIndex(effect_index, group_index),
                ) => {
                    if let Some(effect) =
                        selected_scene_effect(project, self.selected_scene_instance, effect_index)
                    {
                        effect.group_index = group_index;
                    }
                }
                (
                    Some(project),
                    UiAction::SetSceneEffectAnimation(target, effect_index, animation),
                ) => {
                    if let Some(effect) = scene_effect_by_target(project, target, effect_index) {
                        effect.animation = animation;
                    }
                }
                (
                    Some(project),
                    UiAction::SetSelectedSceneEffectAnimation(effect_index, animation),
                ) => {
                    if let Some(effect) =
                        selected_scene_effect(project, self.selected_scene_instance, effect_index)
                    {
                        effect.animation = animation;
                    }
                }
                (
                    Some(project),
                    UiAction::SetSceneEffectAnimationConfigU32(
                        target,
                        effect_index,
                        config_index,
                        value,
                    ),
                ) => {
                    if let Some(effect) = scene_effect_by_target(project, target, effect_index)
                        && let Some(config) = effect.animation_config.u32(config_index)
                    {
                        *config = value;
                    }
                }
                (
                    Some(project),
                    UiAction::SetSelectedSceneEffectAnimationConfigU32(
                        effect_index,
                        config_index,
                        value,
                    ),
                ) => {
                    if let Some(effect) =
                        selected_scene_effect(project, self.selected_scene_instance, effect_index)
                        && let Some(config) = effect.animation_config.u32(config_index)
                    {
                        *config = value;
                    }
                }
                (
                    Some(project),
                    UiAction::SetSceneEffectAnimationConfigF32(
                        target,
                        effect_index,
                        config_index,
                        value,
                    ),
                ) => {
                    if let Some(effect) = scene_effect_by_target(project, target, effect_index)
                        && let Some(config) = effect.animation_config.float(config_index)
                    {
                        *config = FloatValue::F32(value);
                    }
                }
                (
                    Some(project),
                    UiAction::SetSelectedSceneEffectAnimationConfigF32(
                        effect_index,
                        config_index,
                        value,
                    ),
                ) => {
                    if let Some(effect) =
                        selected_scene_effect(project, self.selected_scene_instance, effect_index)
                        && let Some(config) = effect.animation_config.float(config_index)
                    {
                        *config = FloatValue::F32(value);
                    }
                }
                (Some(project), UiAction::AddSceneEffect(target)) => {
                    if let Some(scene_instance) =
                        project.scene_instance_by_location_or_quick_index(target)
                    {
                        scene_instance.scene.add_effect(Default::default());
                    }
                }
                (Some(project), UiAction::AddSelectedSceneEffect) => {
                    if let Some(scene_instance) =
                        project.get_scenes_instance(&self.selected_scene_instance)
                    {
                        scene_instance.scene.add_effect(Default::default());
                    }
                }
                (Some(project), UiAction::RemoveSceneEffect(target, effect_index)) => {
                    if let Some(scene_instance) =
                        project.scene_instance_by_location_or_quick_index(target)
                    {
                        scene_instance.scene.remove_effect(effect_index);
                    }
                }
                (Some(project), UiAction::RemoveSelectedSceneEffect(effect_index)) => {
                    if let Some(scene_instance) =
                        project.get_scenes_instance(&self.selected_scene_instance)
                    {
                        scene_instance.scene.remove_effect(effect_index);
                    }
                }
                (Some(project), UiAction::CloneSceneEffect(target, effect_index)) => {
                    if let Some(scene_instance) =
                        project.scene_instance_by_location_or_quick_index(target)
                        && let Some(effect) = scene_instance.scene.effect(effect_index).cloned()
                    {
                        scene_instance.scene.add_effect(effect);
                    }
                }
                (Some(project), UiAction::CloneSelectedSceneEffect(effect_index)) => {
                    if let Some(scene_instance) =
                        project.get_scenes_instance(&self.selected_scene_instance)
                        && let Some(effect) = scene_instance.scene.effect(effect_index).cloned()
                    {
                        scene_instance.scene.add_effect(effect);
                    }
                }
                (_, UiAction::SetAnimationShaderCode(animation_id, shader_code)) => {
                    update_animation(&mut self.collections, animation_id, |animation| {
                        animation.shader_code = shader_code;
                    });
                }
                (_, UiAction::AddAnimationArgument(animation_id)) => {
                    update_animation(&mut self.collections, animation_id, |animation| {
                        animation.arguments.push(Argument::default());
                    });
                }
                (_, UiAction::RemoveAnimationArgument(animation_id, argument_index)) => {
                    update_animation(&mut self.collections, animation_id, |animation| {
                        if argument_index < animation.arguments.len() {
                            animation.arguments.remove(argument_index);
                        }
                    });
                }
                (_, UiAction::SetAnimationArgumentName(animation_id, argument_index, name)) => {
                    update_animation(&mut self.collections, animation_id, |animation| {
                        if let Some(argument) = animation.arguments.get_mut(argument_index) {
                            argument.name = name;
                        }
                    });
                }
                (_, UiAction::SetAnimationArgumentKind(animation_id, argument_index, kind)) => {
                    update_animation(&mut self.collections, animation_id, |animation| {
                        if let Some(argument) = animation.arguments.get_mut(argument_index) {
                            argument.kind = ArgumentKind::from(kind);
                        }
                    });
                }
                (_, UiAction::Tap) => {
                    self.timing.tap();
                }
                (_, UiAction::SpeedAdd(delta)) => {
                    self.timing.add_speed(delta);
                }
                (_, UiAction::SpeedMultiply(multiplier)) => {
                    self.timing.multiply_speed(multiplier);
                }
                (_, UiAction::SetBlackout(blackout)) => {
                    self.blackout = blackout;
                }
                (_, UiAction::SetProject(project)) => {
                    if let Some(project) = Asset::get(project, &self.collections) {
                        self.persistant_state.set_last_project_id(project.id);
                        self.persistant_state.save();

                        self.project_id = Some(project.id);
                        let project = Arc::unwrap_or_clone(project).data;
                        self.set_selected_scene_instance(
                            project
                                .all_scene_instance_locations()
                                .next()
                                .copied()
                                .unwrap_or_default(),
                        );
                        self.extract_output.routings = project.output_routings.clone();
                        *ARTNET_CONFIG.lock() = project.artnet_config.clone();
                        project.channel_overwrites.clone().set();
                        self.audio_pool.selected_device = project.audio_input_device.clone();
                        self.audio_pool.restart_fft();
                        self.project = Some(project);
                        UiAction::ReloadShaderCode(None).enqueue();
                    } else {
                        self.windows.external_device_settings.close();
                        self.windows.output_routings.close();
                        self.windows.shortcuts.close();
                        self.project.take();
                        self.project_id.take();
                    };
                    Svg::reset();
                }
                (_, UiAction::CloseWindow(viewport_id)) => {
                    self.other_main_windows.remove(&viewport_id);
                }
                (_, UiAction::OpenGitConfigWindow) => {
                    self.windows.git_config.open();
                }
                (_, UiAction::MidiOutputActive(active)) => {
                    self.midi_output_active = active;
                }
                (
                    Some(_),
                    UiAction::SetProjectPalette(_)
                    | UiAction::SetProjectPaletteFromAsset(_)
                    | UiAction::SetProjectPalettePrimary(_)
                    | UiAction::SetProjectPaletteSecondary(_)
                    | UiAction::SetProjectPaletteGradient(_, _)
                    | UiAction::SetScenePaletteOverwrite(_, _)
                    | UiAction::SetSelectedScenePaletteOverwrite(_)
                    | UiAction::SetScenePaletteOverwriteFromAsset(_, _)
                    | UiAction::SetSelectedScenePaletteOverwriteFromAsset(_)
                    | UiAction::SetScenePaletteOverwritePrimary(_, _)
                    | UiAction::SetSelectedScenePaletteOverwritePrimary(_)
                    | UiAction::SetScenePaletteOverwriteSecondary(_, _)
                    | UiAction::SetSelectedScenePaletteOverwriteSecondary(_)
                    | UiAction::SetScenePaletteOverwriteGradient(_, _, _)
                    | UiAction::SetSelectedScenePaletteOverwriteGradient(_, _),
                ) => {}

                (_, UiAction::Error(error)) => {
                    log::error!("{error}");

                    if let Err(err) = Notification::new()
                        .summary("Error")
                        .body(&error)
                        .icon("application-gled")
                        .show()
                    {
                        log::error!("Could not show notification: {err:?}");
                    }

                    self.windows.errors.entries.push(error);
                }
                (None, _) => log::trace!("Ingoring ui action which needs a loaded project"),
            }
        }
    }
}

