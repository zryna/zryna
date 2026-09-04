use std::collections::BTreeSet;

use zryna_layout::{self as layout, TypeCategory};

use super::super::Ty;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PreparationRoute {
    Aggregate,
    MixedSummary,
    LegacyVec,
    Unsupported,
}

pub(super) fn supported(ty: Ty, layouts: &layout::VerifiedLayouts) -> bool {
    let mut visited = BTreeSet::new();
    let mut pending = vec![ty.layout];
    while let Some(id) = pending.pop() {
        if !visited.insert(id) {
            continue;
        }
        let Some(record) = layouts.type_by_id(id) else { return false };
        match record.category() {
            TypeCategory::Bool | TypeCategory::I32 | TypeCategory::String => {}
            TypeCategory::Struct => pending.extend(record.fields().iter().map(|field| field.ty())),
            TypeCategory::Enum => {
                pending.extend(record.variants().iter().filter_map(|v| v.payload()));
            }
            TypeCategory::FixedArray | TypeCategory::Vec => {
                let Some(element) = record.referenced_type() else { return false };
                if record.category() == TypeCategory::Vec
                    && layouts.type_by_id(element).is_none_or(|element| element.size() == 0)
                {
                    return false;
                }
                pending.push(element);
            }
            TypeCategory::Shared | TypeCategory::Weak => return false,
        }
    }
    true
}

pub(super) fn requires_summary(ty: Ty, layouts: &layout::VerifiedLayouts) -> bool {
    route(ty, layouts) == PreparationRoute::MixedSummary
}

pub(super) fn route(ty: Ty, layouts: &layout::VerifiedLayouts) -> PreparationRoute {
    let legacy = match ty.category {
        TypeCategory::Enum => super::owned_enum_graph_is_supported(ty, layouts),
        TypeCategory::Vec => layouts
            .type_by_id(ty.layout)
            .and_then(zryna_layout::VerifiedType::referenced_type)
            .and_then(|id| layouts.type_by_id(id))
            .is_some_and(|element| {
                matches!(
                    element.category(),
                    TypeCategory::Bool | TypeCategory::I32 | TypeCategory::String
                )
            }),
        _ => super::aggregate_graph_is_supported(ty, layouts, &mut BTreeSet::new()),
    };
    if legacy {
        if ty.category == TypeCategory::Vec {
            PreparationRoute::LegacyVec
        } else {
            PreparationRoute::Aggregate
        }
    } else if supported(ty, layouts) {
        PreparationRoute::MixedSummary
    } else {
        PreparationRoute::Unsupported
    }
}

impl super::PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn local_preparation_route(&self, ty: Ty) -> PreparationRoute {
        if self.mixed_function && supported(ty, self.layouts) {
            PreparationRoute::MixedSummary
        } else {
            route(ty, self.layouts)
        }
    }
}
