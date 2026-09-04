use std::collections::BTreeMap;

use zryna_ir::data_ownership_v1::{self as ir, raw};
use zryna_source::Span;

use super::super::{Errors, Ty};

pub(super) struct ProjectionDescriptor {
    pub(super) ty: Ty,
    pub(super) at: Span,
    pub(super) key: (u32, u8, u32),
    pub(super) kind: raw::PlaceKind,
}

pub(super) trait ProjectionTopology {
    fn cached(&self, key: (u32, u8, u32)) -> Option<raw::PlaceId>;
    fn used_places(&self) -> usize;
    fn insert(&mut self, descriptor: ProjectionDescriptor) -> Option<raw::PlaceId>;
    fn checks_capacity(&self) -> bool {
        true
    }
}

pub(super) fn projection_capacity(used: usize, at: Span, errors: &mut Errors<'_>) -> bool {
    if used >= ir::MAX_PLACES_PER_FUNCTION {
        errors.at(
            "ZRYNA-M3201",
            at,
            "derived owned projection places exceed the per-function M3 limit",
            "reduce distinct private aggregate field and fixed-array projections",
        );
        return false;
    }
    true
}

pub(super) fn project(
    topology: &mut impl ProjectionTopology,
    descriptor: ProjectionDescriptor,
    errors: &mut Errors<'_>,
) -> Option<raw::PlaceId> {
    if let Some(place) = topology.cached(descriptor.key) {
        return Some(place);
    }
    if topology.checks_capacity()
        && !projection_capacity(topology.used_places(), descriptor.at, errors)
    {
        return None;
    }
    topology.insert(descriptor)
}

pub(super) struct MaterializedProjectionTopology<'a> {
    pub(super) projections: &'a mut BTreeMap<(u32, u8, u32), raw::PlaceId>,
    pub(super) places: &'a mut Vec<raw::Place>,
    pub(super) reserved_places: usize,
}

impl ProjectionTopology for MaterializedProjectionTopology<'_> {
    fn cached(&self, key: (u32, u8, u32)) -> Option<raw::PlaceId> {
        self.projections.get(&key).copied()
    }

    fn used_places(&self) -> usize {
        self.places.len().saturating_add(self.reserved_places)
    }

    fn insert(&mut self, descriptor: ProjectionDescriptor) -> Option<raw::PlaceId> {
        let place = raw::PlaceId(u32::try_from(self.places.len()).ok()?);
        self.places.push(raw::Place {
            id: place,
            ty: descriptor.ty.ir,
            span: descriptor.at,
            kind: descriptor.kind,
        });
        self.projections.insert(descriptor.key, place);
        Some(place)
    }
}
