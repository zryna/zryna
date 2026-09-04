use super::super::OwnerState;
use std::collections::BTreeSet;
use zryna_ir::data_ownership_v1::raw;

pub(super) struct AvailabilityView<'a, P> {
    owners: &'a OwnerState,
    moved_projections: &'a BTreeSet<raw::PlaceId>,
    partial_roots: &'a BTreeSet<raw::PlaceId>,
    parent: P,
}

impl<'a, P: Fn(raw::PlaceId) -> Option<raw::PlaceId>> AvailabilityView<'a, P> {
    pub(super) fn new(
        owners: &'a OwnerState,
        moved_projections: &'a BTreeSet<raw::PlaceId>,
        partial_roots: &'a BTreeSet<raw::PlaceId>,
        parent: P,
    ) -> Self {
        Self { owners, moved_projections, partial_roots, parent }
    }
    pub(super) fn place_is_at_or_below(&self, mut place: raw::PlaceId, root: raw::PlaceId) -> bool {
        let mut visited = BTreeSet::new();
        while visited.insert(place) {
            if place == root {
                return true;
            }
            let Some(parent) = (self.parent)(place) else { return false };
            place = parent;
        }
        false
    }
    pub(super) fn places_overlap(&self, left: raw::PlaceId, right: raw::PlaceId) -> bool {
        self.place_is_at_or_below(left, right) || self.place_is_at_or_below(right, left)
    }
    pub(super) fn whole_root_available(&self, root: raw::PlaceId) -> bool {
        self.owners.contains(root) && !self.partial_roots.contains(&root)
    }
    pub(super) fn projection_available(
        &self,
        projection: raw::PlaceId,
        root: raw::PlaceId,
    ) -> bool {
        self.owners.contains(root)
            && !self.moved_projections.iter().any(|moved| self.places_overlap(*moved, projection))
    }
}

pub(super) fn materialized_availability<'a>(
    owners: &'a OwnerState,
    moved_projections: &'a BTreeSet<raw::PlaceId>,
    partial_roots: &'a BTreeSet<raw::PlaceId>,
    places: &'a [raw::Place],
) -> AvailabilityView<'a, impl Fn(raw::PlaceId) -> Option<raw::PlaceId> + 'a> {
    AvailabilityView::new(owners, moved_projections, partial_roots, move |place| {
        parent_kind(&places.get(place.0 as usize)?.kind)
    })
}

pub(super) fn parent_kind(kind: &raw::PlaceKind) -> Option<raw::PlaceId> {
    match kind {
        raw::PlaceKind::StructField { base, .. }
        | raw::PlaceKind::EnumPayload { base, .. }
        | raw::PlaceKind::FixedArrayConstant { base, .. } => Some(*base),
        raw::PlaceKind::Parameter(_) | raw::PlaceKind::Local(_) | raw::PlaceKind::Temporary(_) => {
            None
        }
    }
}
