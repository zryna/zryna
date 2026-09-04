use std::collections::BTreeSet;

use zryna_ir::data_ownership_v1::{self as ir, raw};
use zryna_layout::{self as layout, TypeCategory};
use zryna_source::Span;
use zryna_syntax::v4 as syntax;

use super::super::{OwnedProjectionShapeEntry, OwnedStaticProjectionKind, Ty};
use super::PrivateOwnedAggregateLowerer;

pub(in crate::data_ownership_v1) fn aggregate_graph_is_supported(
    ty: Ty,
    layouts: &layout::VerifiedLayouts,
    visiting: &mut BTreeSet<layout::TypeId>,
) -> bool {
    if !visiting.insert(ty.layout) {
        return false;
    }
    let supported = layouts.type_by_id(ty.layout).is_some_and(|record| match record.category() {
        TypeCategory::Bool | TypeCategory::I32 | TypeCategory::String => true,
        TypeCategory::Struct => record.fields().iter().all(|field| {
            let child = layouts.type_by_id(field.ty()).map(|child| Ty {
                layout: child.id(),
                ir: raw::TypeId(child.id().index()),
                category: child.category(),
                drop_kind: child.drop_kind(),
                runtime_kind: child.runtime_kind(),
                cloneable: false,
            });
            child.is_some_and(|child| aggregate_graph_is_supported(child, layouts, visiting))
        }),
        TypeCategory::FixedArray => record.referenced_type().is_some_and(|child| {
            layouts.type_by_id(child).is_some_and(|child| {
                aggregate_graph_is_supported(
                    Ty {
                        layout: child.id(),
                        ir: raw::TypeId(child.id().index()),
                        category: child.category(),
                        drop_kind: child.drop_kind(),
                        runtime_kind: child.runtime_kind(),
                        cloneable: false,
                    },
                    layouts,
                    visiting,
                )
            })
        }),
        TypeCategory::Enum | TypeCategory::Vec | TypeCategory::Shared | TypeCategory::Weak => false,
    });
    visiting.remove(&ty.layout);
    supported
}

pub(super) fn owned_enum_graph_is_supported(ty: Ty, layouts: &layout::VerifiedLayouts) -> bool {
    layouts.type_by_id(ty.layout).is_some_and(|record| {
        record.category() == TypeCategory::Enum
            && record.variants().iter().all(|variant| {
                variant.payload().is_none_or(|payload| {
                    layouts.type_by_id(payload).is_some_and(|payload| {
                        aggregate_graph_is_supported(
                            Ty {
                                layout: payload.id(),
                                ir: raw::TypeId(payload.id().index()),
                                category: payload.category(),
                                drop_kind: payload.drop_kind(),
                                runtime_kind: payload.runtime_kind(),
                                cloneable: false,
                            },
                            layouts,
                            &mut BTreeSet::new(),
                        )
                    })
                })
            })
    })
}

fn append_owned_projection_shape(
    ty: Ty,
    layouts: &layout::VerifiedLayouts,
    parent: Option<usize>,
    shape: &mut Vec<OwnedProjectionShapeEntry>,
) -> Option<()> {
    let record = layouts.type_by_id(ty.layout)?;
    match ty.category {
        TypeCategory::Struct => {
            for field in record.fields() {
                if shape.len() >= ir::MAX_PLACES_PER_FUNCTION {
                    return None;
                }
                let child_record = layouts.type_by_id(field.ty())?;
                let child = Ty {
                    layout: child_record.id(),
                    ir: raw::TypeId(child_record.id().index()),
                    category: child_record.category(),
                    drop_kind: child_record.drop_kind(),
                    runtime_kind: child_record.runtime_kind(),
                    cloneable: false,
                };
                let index = shape.len();
                shape.push(OwnedProjectionShapeEntry {
                    parent,
                    ty: child,
                    kind: OwnedStaticProjectionKind::StructField { ordinal: field.ordinal() },
                });
                append_owned_projection_shape(child, layouts, Some(index), shape)?;
            }
        }
        TypeCategory::FixedArray => {
            let child_record = layouts.type_by_id(record.referenced_type()?)?;
            let child = Ty {
                layout: child_record.id(),
                ir: raw::TypeId(child_record.id().index()),
                category: child_record.category(),
                drop_kind: child_record.drop_kind(),
                runtime_kind: child_record.runtime_kind(),
                cloneable: false,
            };
            let length = usize::try_from(record.array_length()?).ok()?;
            for index in 0..length {
                if shape.len() >= ir::MAX_PLACES_PER_FUNCTION {
                    return None;
                }
                let ordinal = u32::try_from(index).ok()?;
                let child_index = shape.len();
                shape.push(OwnedProjectionShapeEntry {
                    parent,
                    ty: child,
                    kind: OwnedStaticProjectionKind::FixedArrayConstant { index: ordinal },
                });
                append_owned_projection_shape(child, layouts, Some(child_index), shape)?;
            }
        }
        TypeCategory::Bool | TypeCategory::I32 | TypeCategory::String => {}
        TypeCategory::Enum | TypeCategory::Vec | TypeCategory::Shared | TypeCategory::Weak => {
            return None;
        }
    }
    Some(())
}

pub(in crate::data_ownership_v1) fn complete_owned_projection_shape(
    ty: Ty,
    layouts: &layout::VerifiedLayouts,
) -> Option<Vec<OwnedProjectionShapeEntry>> {
    let mut shape = Vec::new();
    append_owned_projection_shape(ty, layouts, None, &mut shape)?;
    Some(shape)
}

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn expression(&self, id: u32) -> Option<&syntax::RawExpressionSyntax> {
        usize::try_from(id).ok().and_then(|index| self.function.body.expressions.get(index))
    }

    pub(super) fn supported(&self, ty: Ty) -> bool {
        if ty.category == TypeCategory::Enum {
            owned_enum_graph_is_supported(ty, self.layouts)
        } else {
            aggregate_graph_is_supported(ty, self.layouts, &mut BTreeSet::new())
        }
    }

    fn place_parent(&self, place: raw::PlaceId) -> Option<raw::PlaceId> {
        match self.places.get(place.0 as usize)?.kind {
            raw::PlaceKind::StructField { base, .. }
            | raw::PlaceKind::EnumPayload { base, .. }
            | raw::PlaceKind::FixedArrayConstant { base, .. } => Some(base),
            raw::PlaceKind::Parameter(_)
            | raw::PlaceKind::Local(_)
            | raw::PlaceKind::Temporary(_) => None,
        }
    }

    pub(super) fn place_is_at_or_below(&self, mut place: raw::PlaceId, root: raw::PlaceId) -> bool {
        let mut visited = BTreeSet::new();
        while visited.insert(place) {
            if place == root {
                return true;
            }
            let Some(parent) = self.place_parent(place) else { return false };
            place = parent;
        }
        false
    }

    fn places_overlap(&self, left: raw::PlaceId, right: raw::PlaceId) -> bool {
        self.place_is_at_or_below(left, right) || self.place_is_at_or_below(right, left)
    }

    pub(super) fn complete_projection_shape(
        &self,
        ty: Ty,
    ) -> Option<Vec<OwnedProjectionShapeEntry>> {
        complete_owned_projection_shape(ty, self.layouts)
    }

    pub(super) fn existing_projection_shape(
        &self,
        root: raw::PlaceId,
        shape: &[OwnedProjectionShapeEntry],
    ) -> Vec<Option<raw::PlaceId>> {
        let mut places = Vec::with_capacity(shape.len());
        for entry in shape {
            let parent = match entry.parent {
                Some(index) => places[index],
                None => Some(root),
            };
            places.push(parent.and_then(|parent| {
                let key = match entry.kind {
                    OwnedStaticProjectionKind::StructField { ordinal } => (parent.0, 0, ordinal),
                    OwnedStaticProjectionKind::FixedArrayConstant { index } => (parent.0, 1, index),
                };
                self.projections.get(&key).copied()
            }));
        }
        places
    }

    pub(super) fn materialize_projection_shape(
        &mut self,
        root: raw::PlaceId,
        shape: &[OwnedProjectionShapeEntry],
        at: Span,
    ) -> Vec<raw::PlaceId> {
        let mut places = Vec::with_capacity(shape.len());
        for entry in shape {
            let parent = entry.parent.map_or(root, |index| places[index]);
            let (key, kind) = match entry.kind {
                OwnedStaticProjectionKind::StructField { ordinal } => {
                    ((parent.0, 0, ordinal), raw::PlaceKind::StructField { base: parent, ordinal })
                }
                OwnedStaticProjectionKind::FixedArrayConstant { index } => (
                    (parent.0, 1, index),
                    raw::PlaceKind::FixedArrayConstant { base: parent, index },
                ),
            };
            places.push(
                self.push_projection(entry.ty, at, key, kind)
                    .expect("partial transfer topology capacity preflighted"),
            );
        }
        places
    }

    pub(super) fn migrate_partial_mask(
        &mut self,
        source_root: raw::PlaceId,
        target_root: raw::PlaceId,
        source_places: &[raw::PlaceId],
        target_places: &[raw::PlaceId],
    ) {
        assert_eq!(source_places.len(), target_places.len(), "partial transfer topology matched");
        assert!(self.partial_roots.contains(&source_root), "partial transfer source tracked");
        let moved = self
            .moved_projections
            .iter()
            .copied()
            .filter(|place| self.place_is_at_or_below(*place, source_root))
            .collect::<Vec<_>>();
        let mapped = moved
            .iter()
            .map(|source| {
                source_places
                    .iter()
                    .position(|candidate| candidate == source)
                    .map(|index| target_places[index])
            })
            .collect::<Option<Vec<_>>>()
            .expect("complete partial transfer mask mapping");
        assert!(self.partial_roots.remove(&source_root), "partial transfer source tracked");
        self.partial_roots.insert(target_root);
        for (source, target) in moved.into_iter().zip(mapped) {
            self.moved_projections.remove(&source);
            self.moved_projections.insert(target);
        }
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

    pub(super) fn push_projection(
        &mut self,
        ty: Ty,
        at: Span,
        key: (u32, u8, u32),
        kind: raw::PlaceKind,
    ) -> Option<raw::PlaceId> {
        let reserved_places = self.reserved_constructor_places();
        let mut topology = super::projection_topology::MaterializedProjectionTopology {
            projections: &mut self.projections,
            places: &mut self.places,
            reserved_places,
        };
        super::projection_topology::project(
            &mut topology,
            super::projection_topology::ProjectionDescriptor { ty, at, key, kind },
            self.errors,
        )
    }
}
