use crate::{
    midi::learn::{LearnState, MidiLearnTarget},
    storage::{
        asset::{
            Asset,
            midi_controller::{MidiController, MidiOutputBindingKind},
        },
        collections::Collections,
    },
};

pub(super) fn apply_learn_captures(
    controller: &mut Asset<MidiController>,
    dirty: &mut bool,
    collections: &mut Collections,
    learn_state: &mut LearnState,
) {
    let mut changed = false;
    while let Some(capture) = learn_state.pop_capture() {
        if capture.controller_id != controller.id {
            continue;
        }

        match capture.target {
            MidiLearnTarget::InputBinding(binding_index) => {
                if let Some(binding) = controller.data.mapping.input_bindings.get_mut(binding_index) {
                    binding.trigger = capture.trigger;
                    changed = true;
                }
            }
            MidiLearnTarget::OutputBinding(binding_index) => {
                if let Some(binding) = controller.data.mapping.output_bindings.get_mut(binding_index) {
                    match &mut binding.kind {
                        MidiOutputBindingKind::Value(value) => {
                            value.status = capture.trigger.status;
                            value.data1 = capture.trigger.data1;
                        }
                        MidiOutputBindingKind::SceneColorValue(scene_color_value) => {
                            scene_color_value.status = capture.trigger.status;
                            scene_color_value.data1 = capture.trigger.data1;
                        }
                    }
                    changed = true;
                }
            }
        }
    }

    if changed {
        *dirty = true;
        controller.clone().save(collections);
    }
}
