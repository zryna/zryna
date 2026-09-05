use std::collections::{BTreeMap, BTreeSet};
use zryna_ir::data_ownership_v1::raw;
use zryna_layout::{self as layout, TypeCategory};
use zryna_source::Span;
use zryna_syntax::v4 as syntax;

use super::super::type_model::{OwnedAggregatePlace, ProjectedAggregateMoveContext};
use super::super::{Binding, Errors, SemanticInput, Ty};
use super::availability::{AvailabilityView, materialized_availability};
use super::{
    PrivateOwnedAggregateLowerer, aggregate_graph_is_supported, owned_enum_graph_is_supported,
};
use crate::data_ownership_v1::diagnostics::span;

pub(super) enum ReferenceKind {
    Copy,
    Move,
}
pub(super) struct ReferenceDecision {
    pub(super) binding: Binding,
    pub(super) kind: ReferenceKind,
}
pub(super) enum ProjectionOperation {
    Copy,
    Move { aggregate_subobject: bool },
}

pub(super) struct OperandDecisions<'a, 'f, 's, P> {
    pub(super) input: SemanticInput<'a>,
    pub(super) function: &'f syntax::RawFunctionSyntax,
    pub(super) bindings: &'s BTreeMap<String, Binding>,
    pub(super) layouts: &'a layout::VerifiedLayouts,
    pub(super) availability: AvailabilityView<'s, P>,
    pub(super) aggregate_subobject_moves: usize,
    pub(super) errors: &'s mut Errors<'a>,
}

pub(super) fn supported(ty: Ty, layouts: &layout::VerifiedLayouts) -> bool {
    if ty.category == TypeCategory::Enum {
        owned_enum_graph_is_supported(ty, layouts)
    } else {
        aggregate_graph_is_supported(ty, layouts, &mut BTreeSet::new())
    }
}

impl<P: Fn(raw::PlaceId) -> Option<raw::PlaceId>> OperandDecisions<'_, '_, '_, P> {
    pub(super) fn supported(&self, ty: Ty) -> bool {
        supported(ty, self.layouts)
    }
    pub(super) fn expression(&self, id: u32) -> Option<&syntax::RawExpressionSyntax> {
        usize::try_from(id).ok().and_then(|index| self.function.body.expressions.get(index))
    }
    pub(super) fn reference_decision(
        &mut self,
        name: &syntax::RawIdentifierSyntax,
        expected: Ty,
    ) -> Option<ReferenceDecision> {
        let Some(binding) = self.bindings.get(&name.text).cloned() else {
            let wrong_case = self.bindings.keys().any(|key| key.eq_ignore_ascii_case(&name.text));
            self.errors.at(
                "ZRYNA-M3002",
                span(self.input.sources(), name.span),
                if wrong_case {
                    format!("aggregate value '{}' has the wrong portable ASCII case", name.text)
                } else {
                    format!("aggregate value '{}' is not declared", name.text)
                },
                "reference one exact preceding local using its declared spelling",
            );
            return None;
        };
        if binding.ty != expected {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), name.span),
                "aggregate operand has the wrong exact type",
                "use the exact declared field, element, local, or result type",
            );
            return None;
        }
        if expected.is_copy() {
            return Some(ReferenceDecision { binding, kind: ReferenceKind::Copy });
        }
        if !self.availability.whole_root_available(binding.place) {
            self.errors.at(
                "ZRYNA-M3014",
                span(self.input.sources(), name.span),
                format!("aggregate value '{}' is moved or only partially available", name.text),
                "move a whole owned aggregate only before moving any of its projections",
            );
            return None;
        }
        Some(ReferenceDecision { binding, kind: ReferenceKind::Move })
    }
    pub(super) fn preflight_aggregate_subobject_move_site(&mut self, at: Span) -> bool {
        if self.aggregate_subobject_moves == 0 {
            return true;
        }
        self.errors.at(
            "ZRYNA-M3016",
            at,
            "this checkpoint admits only one aggregate-subobject move per function",
            "move one supported Struct or fixed-array subobject into one exact direct local",
        );
        false
    }
    pub(super) fn projection_decision(
        &mut self,
        projection: OwnedAggregatePlace,
        expected: Ty,
        aggregate_context: Option<ProjectedAggregateMoveContext>,
        at: Span,
    ) -> Option<ProjectionOperation> {
        if projection.is_root || projection.ty != expected {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                "owned projection has the wrong exact contextual type",
                "use one exact supported Struct field or fixed-array element",
            );
            return None;
        }
        if !self.availability.projection_available(projection.place, projection.root) {
            self.errors.at(
                "ZRYNA-M3014",
                at,
                "owned projection is unavailable or overlaps an already moved subobject",
                "move each owned field or fixed-array element at most once",
            );
            return None;
        }
        if expected.is_copy() {
            return Some(ProjectionOperation::Copy);
        }
        let aggregate_subobject =
            matches!(expected.category, TypeCategory::Struct | TypeCategory::FixedArray);
        if aggregate_subobject && aggregate_context.is_none() {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                "static aggregate-subobject move requires one exact direct local or final return",
                "initialize one exact private local or return the exact result type from the Struct field or constant fixed-array element",
            );
            return None;
        }
        if aggregate_subobject && !self.preflight_aggregate_subobject_move_site(at) {
            return None;
        }
        if !matches!(
            expected.category,
            TypeCategory::String | TypeCategory::Struct | TypeCategory::FixedArray
        ) || !self.supported(expected)
        {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                "owned projection type is outside the static subobject move checkpoint",
                "move a String, supported Struct, or supported fixed-array field or constant element into one exact direct local",
            );
            return None;
        }
        Some(ProjectionOperation::Move { aggregate_subobject })
    }
}

impl<'a, 'f> PrivateOwnedAggregateLowerer<'a, 'f, '_> {
    pub(super) fn operand_decisions(
        &mut self,
    ) -> OperandDecisions<'a, 'f, '_, impl Fn(raw::PlaceId) -> Option<raw::PlaceId> + '_> {
        OperandDecisions {
            input: self.input,
            function: self.function,
            bindings: &self.bindings,
            layouts: self.layouts,
            availability: materialized_availability(
                &self.owners,
                &self.moved_projections,
                &self.partial_roots,
                &self.places,
            ),
            aggregate_subobject_moves: self.aggregate_subobject_moves,
            errors: self.errors,
        }
    }
}
