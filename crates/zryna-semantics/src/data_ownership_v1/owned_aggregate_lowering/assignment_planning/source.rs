use zryna_ir::data_ownership_v1::raw;
use zryna_layout::TypeCategory;
use zryna_source::Span;
use zryna_syntax::v4::{self as syntax, RawExpressionKind};

use super::super::super::type_model::{ProjectedAggregateAssignmentSource, Ty};
use super::super::PrivateOwnedAggregateLowerer;
use crate::data_ownership_v1::diagnostics::span;

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    fn projected_aggregate_assignment_source(
        &mut self,
        value: u32,
        expected: Ty,
        target_root: raw::PlaceId,
    ) -> Option<ProjectedAggregateAssignmentSource> {
        let expression = self.expression(value).cloned()?;
        let expression_at = span(self.input.sources(), expression.span);
        if let RawExpressionKind::Reference { name } = expression.kind {
            return self.projected_aggregate_assignment_root_source(
                name,
                expression_at,
                expected,
                target_root,
            );
        }

        self.projected_aggregate_assignment_projection_source(
            value,
            expression_at,
            expected,
            target_root,
        )
    }

    fn projected_aggregate_assignment_root_source(
        &mut self,
        name: syntax::RawIdentifierSyntax,
        expression_at: Span,
        expected: Ty,
        target_root: raw::PlaceId,
    ) -> Option<ProjectedAggregateAssignmentSource> {
        let Some(source) = self.bindings.get(&name.text).cloned() else {
            self.errors.at(
                "ZRYNA-M3002",
                span(self.input.sources(), name.span),
                format!("aggregate value '{}' is not declared", name.text),
                "reference one exact preceding aggregate local",
            );
            return None;
        };
        if source.ty != expected {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), name.span),
                "projected aggregate assignment source has the wrong exact type",
                "move a whole root or static subobject with the exact target projection type",
            );
            return None;
        }
        if source.place == target_root {
            self.errors.at(
                "ZRYNA-M3014",
                span(self.input.sources(), name.span),
                "projected aggregate assignment cannot consume its enclosing root",
                "move one aggregate root under a distinct enclosing root into the projection",
            );
            return None;
        }
        if !self.whole_root_available(source.place) {
            self.errors.at(
                "ZRYNA-M3014",
                span(self.input.sources(), name.span),
                "projected aggregate assignment source is moved or only partially available",
                "move one distinct fully initialized aggregate root into the projection",
            );
            return None;
        }
        Some(ProjectedAggregateAssignmentSource::MoveRoot { name, at: expression_at })
    }

    fn projected_aggregate_assignment_projection_source(
        &mut self,
        value: u32,
        expression_at: Span,
        expected: Ty,
        target_root: raw::PlaceId,
    ) -> Option<ProjectedAggregateAssignmentSource> {
        let Some(source) = self.owned_place_preflight(value) else {
            self.errors.at(
                "ZRYNA-M3013",
                expression_at,
                "projected aggregate assignment requires one whole root or static aggregate subobject source",
                "move one distinct fully initialized exact aggregate root or canonical Struct field or constant fixed-array element into the projection",
            );
            return None;
        };
        if source.place.is_root
            || source.place.ty.is_copy()
            || !matches!(source.place.ty.category, TypeCategory::Struct | TypeCategory::FixedArray)
            || !self.supported(source.place.ty)
        {
            self.errors.at(
                "ZRYNA-M3013",
                expression_at,
                "projected aggregate assignment source is outside the static non-Copy subobject checkpoint",
                "move one supported Struct field or constant fixed-array aggregate element",
            );
            return None;
        }
        if source.place.ty != expected {
            self.errors.at(
                "ZRYNA-M3016",
                expression_at,
                "projected aggregate assignment source has the wrong exact type",
                "move a static subobject with the exact target projection type",
            );
            return None;
        }
        if source.place.root == target_root {
            self.errors.at(
                "ZRYNA-M3014",
                expression_at,
                "projected aggregate assignment source and target require distinct enclosing roots",
                "move a static aggregate subobject between two distinct local owners",
            );
            return None;
        }
        if !self.preflight_projection_available(&source) {
            self.errors.at(
                "ZRYNA-M3014",
                expression_at,
                "projected aggregate assignment source subobject is moved or overlaps a moved projection",
                "move one initialized available static aggregate subobject",
            );
            return None;
        }
        if !self.preflight_aggregate_subobject_move_site(expression_at) {
            return None;
        }
        let Some(shape) = self.complete_projection_shape(expected) else {
            self.errors.at(
                "ZRYNA-M3016",
                expression_at,
                "projected aggregate assignment source has no finite static topology",
                "move an acyclic supported Struct or fixed-array subobject",
            );
            return None;
        };
        let existing = self.existing_projection_shape(source.place.place, &shape);
        let missing_descendant_places = existing.iter().filter(|place| place.is_none()).count();
        Some(ProjectedAggregateAssignmentSource::MoveProjection {
            expression: value,
            missing_path_places: source.missing,
            missing_descendant_places,
        })
    }

    fn projected_aggregate_clone_assignment_source(
        &mut self,
        operand: u32,
        expected: Ty,
        target_root: raw::PlaceId,
        clone_span: Span,
    ) -> Option<ProjectedAggregateAssignmentSource> {
        let expression = self.expression(operand).cloned()?;
        if let RawExpressionKind::Reference { name } = expression.kind {
            return self.projected_aggregate_clone_root_assignment_source(
                &name,
                expected,
                target_root,
                clone_span,
            );
        }

        let Some(source) = self.owned_place_preflight(operand) else {
            self.errors.at(
                "ZRYNA-M3013",
                span(self.input.sources(), expression.span),
                "projected aggregate clone assignment requires one whole root or static aggregate subobject source",
                "clone one distinct fully initialized exact aggregate root or canonical Struct field or constant fixed-array element into the projection",
            );
            return None;
        };
        if source.place.is_root
            || source.place.ty.is_copy()
            || !matches!(source.place.ty.category, TypeCategory::Struct | TypeCategory::FixedArray)
            || !self.supported(source.place.ty)
        {
            self.errors.at(
                "ZRYNA-M3013",
                span(self.input.sources(), expression.span),
                "projected aggregate clone assignment source is outside the static non-Copy subobject checkpoint",
                "clone one supported Struct field or constant fixed-array aggregate element",
            );
            return None;
        }
        if source.place.ty != expected {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), expression.span),
                "projected aggregate clone assignment source has the wrong exact type",
                "clone a static subobject with the exact target projection type",
            );
            return None;
        }
        if source.place.root == target_root {
            self.errors.at(
                "ZRYNA-M3014",
                span(self.input.sources(), expression.span),
                "projected aggregate clone assignment source and target require distinct enclosing roots",
                "clone a static aggregate subobject between two distinct local owners",
            );
            return None;
        }
        if !self.preflight_projection_available(&source) {
            self.errors.at(
                "ZRYNA-M3014",
                span(self.input.sources(), expression.span),
                "projected aggregate clone assignment source subobject is moved or overlaps a moved projection",
                "clone one initialized available static aggregate subobject",
            );
            return None;
        }
        if !self.function.parameters.is_empty() {
            self.errors.at(
                "ZRYNA-M3016",
                clone_span,
                "projected aggregate clone assignment does not admit function parameters",
                "clone one static aggregate subobject in a parameter-free private straight-line function",
            );
            return None;
        }
        Some(ProjectedAggregateAssignmentSource::CloneProjection {
            expression: operand,
            at: clone_span,
            missing_path_places: source.missing,
        })
    }

    fn projected_aggregate_clone_root_assignment_source(
        &mut self,
        name: &syntax::RawIdentifierSyntax,
        expected: Ty,
        target_root: raw::PlaceId,
        clone_span: Span,
    ) -> Option<ProjectedAggregateAssignmentSource> {
        let Some(source) = self.bindings.get(&name.text).cloned() else {
            self.errors.at(
                "ZRYNA-M3002",
                span(self.input.sources(), name.span),
                format!("aggregate value '{}' is not declared", name.text),
                "clone one exact preceding aggregate local",
            );
            return None;
        };
        if source.ty != expected {
            self.errors.at(
                "ZRYNA-M3016",
                span(self.input.sources(), name.span),
                "projected aggregate clone assignment source has the wrong exact type",
                "clone a whole root or static subobject with the exact target projection type",
            );
            return None;
        }
        if source.place == target_root {
            self.errors.at(
                "ZRYNA-M3014",
                span(self.input.sources(), name.span),
                "projected aggregate clone assignment source cannot be its enclosing root",
                "clone one distinct aggregate root or static subobject into the projection",
            );
            return None;
        }
        if !self.whole_root_available(source.place) {
            self.errors.at(
                "ZRYNA-M3014",
                span(self.input.sources(), name.span),
                "projected aggregate clone assignment source is moved or only partially available",
                "clone one distinct fully initialized aggregate root into the projection",
            );
            return None;
        }
        Some(ProjectedAggregateAssignmentSource::CloneRoot { binding: source, at: clone_span })
    }

    pub(super) fn projected_aggregate_assignment_value_source(
        &mut self,
        value: u32,
        expected: Ty,
        target_root: raw::PlaceId,
    ) -> Option<ProjectedAggregateAssignmentSource> {
        let expression = self.expression(value).cloned()?;
        if let RawExpressionKind::Clone { value: operand, .. } = expression.kind {
            let clone_span = span(self.input.sources(), expression.span);
            self.projected_aggregate_clone_assignment_source(
                operand,
                expected,
                target_root,
                clone_span,
            )
        } else {
            self.projected_aggregate_assignment_source(value, expected, target_root)
        }
    }
}
