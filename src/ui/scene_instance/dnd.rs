use egui::{
    CursorIcon, DragAndDrop, Frame, Id, InnerResponse, LayerId, Order, Response, Sense, Ui,
    UiBuilder,
};
use std::any::Any;
use std::sync::Arc;

#[doc(alias = "drag and drop")]
pub fn dnd_drag_source<Payload>(
    ui: &mut Ui,
    id: Id,
    payload: Payload,
    add_contents: impl FnOnce(&mut Ui) -> Response,
) -> Response
where
    Payload: Any + Send + Sync,
{
    let is_being_dragged = ui.ctx().is_being_dragged(id);

    if is_being_dragged {
        DragAndDrop::set_payload(ui.ctx(), payload);

        // Paint the body to a new layer:
        let layer_id = LayerId::new(Order::Tooltip, id);
        let InnerResponse {
            inner: _inner,
            response,
        } = ui.scope_builder(UiBuilder::new().layer_id(layer_id), add_contents);

        // Now we move the visuals of the body to where the mouse is.
        // Normally you need to decide a location for a widget first,
        // because otherwise that widget cannot interact with the mouse.
        // However, a dragged component cannot be interacted with anyway
        // (anything with `Order::Tooltip` always gets an empty [`Response`])
        // So this is fine!

        if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
            let delta = pointer_pos - response.rect.center_top();
            ui.ctx()
                .transform_layer_shapes(layer_id, emath::TSTransform::from_translation(delta));
        }

        response
    } else {
        let InnerResponse { inner, response } =
            ui.scope_builder(UiBuilder::new().sense(Sense::click()), add_contents);

        // Check for drags:
        ui.interact(inner.rect, id, Sense::drag())
            .on_hover_cursor(CursorIcon::Grab);

        response
    }
}

#[doc(alias = "drag and drop")]
pub fn dnd_drop_zone<Payload, R>(
    ui: &mut Ui,
    frame: Frame,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> (InnerResponse<R>, Option<Arc<Payload>>)
where
    Payload: Any + Send + Sync,
{
    let is_anything_being_dragged = DragAndDrop::has_any_payload(ui.ctx());
    let can_accept_what_is_being_dragged = DragAndDrop::has_payload_of_type::<Payload>(ui.ctx());

    let mut frame = frame.begin(ui);
    let inner = add_contents(&mut frame.content_ui);
    let response = frame.allocate_space(ui);

    // NOTE: we use `response.contains_pointer` here instead of `hovered`, because
    // `hovered` is always false when another widget is being dragged.
    let style = if is_anything_being_dragged
        && can_accept_what_is_being_dragged
        && response.contains_pointer()
    {
        ui.visuals().widgets.active
    } else {
        ui.visuals().widgets.inactive
    };
    let stroke = style.bg_stroke;

    frame.frame.stroke = stroke;

    frame.paint(ui);

    let payload = response.dnd_release_payload::<Payload>();

    (InnerResponse { inner, response }, payload)
}
