//! Complete static non-handle subobjects reuse the ordinary ownership flow and drop model.

use std::collections::BTreeMap;

use super::{
    Errors, OwnershipFlow, PlaceState, PlaceStateKind, Span, is_projection_below,
    mark_ancestors_partial, ownership_error, pending_slot, projection_path, push_pending_owner,
    raw, root_place, static_projection_path,
};

pub(super) fn valid_type(
    place: raw::PlaceId,
    function: &raw::Function,
    capabilities: &[bool],
) -> bool {
    static_projection_path(place, function)
        && function
            .places
            .get(place.0 as usize)
            .is_some_and(|place| capabilities.get(place.ty.0 as usize).copied().unwrap_or(false))
}

pub(super) fn transfer_complete(
    source: raw::PlaceId,
    destination: raw::PlaceId,
    function: &raw::Function,
    flow: &mut OwnershipFlow,
    span: Span,
    errors: &mut Errors,
) {
    let root = root_place(source, function);
    if flow.states[source.0 as usize].kind != PlaceStateKind::Initialized
        || pending_slot(flow, root).is_none()
        || root == destination
    {
        ownership_error(
            span,
            "generic static move requires one complete pending subobject",
            errors,
        );
        return;
    }
    let variant = flow.variants[source.0 as usize];
    let refinements = function
        .places
        .iter()
        .filter_map(|place| {
            let variant = flow.variants[place.id.0 as usize]?;
            projection_path(place.id, source, &function.places).map(|path| (path, variant))
        })
        .collect::<BTreeMap<_, _>>();
    for place in &function.places {
        if place.id == source || is_projection_below(place.id, source, &function.places) {
            flow.states[place.id.0 as usize] = PlaceState { kind: PlaceStateKind::Moved };
            flow.variants[place.id.0 as usize] = None;
        }
    }
    mark_ancestors_partial(source, function, &mut flow.states);
    push_pending_owner(destination, function, flow, span, errors);
    flow.variants[destination.0 as usize] = variant;
    for place in &function.places {
        let Some(path) = projection_path(place.id, destination, &function.places) else { continue };
        flow.states[place.id.0 as usize] = PlaceState { kind: PlaceStateKind::Initialized };
        flow.variants[place.id.0 as usize] = refinements.get(&path).copied();
    }
}
