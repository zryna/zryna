use zryna_layout::TypeCategory;
use zryna_source::Span;

use super::super::aggregate_resource_formulas::{
    projected_aggregate_assignment_budget_violation,
    projected_aggregate_clone_assignment_budget_violation,
    projected_subobject_assignment_budget_violation,
};
use super::super::type_model::{ProjectedAggregateAssignmentSource, Ty};
use super::PrivateOwnedAggregateLowerer;

mod source;

pub(super) struct ProjectedAggregateAssignmentPlan {
    pub(super) target_ty: Ty,
    pub(super) source: ProjectedAggregateAssignmentSource,
    pub(super) clones_projection: bool,
}

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    fn projected_aggregate_assignment_exceeds_budget(
        &self,
        source: &ProjectedAggregateAssignmentSource,
        missing_path_places: usize,
    ) -> bool {
        match source {
            ProjectedAggregateAssignmentSource::MoveRoot { .. } => {
                projected_aggregate_assignment_budget_violation(
                    self.budget_values(),
                    self.budget_places(),
                    self.instructions.len(),
                    self.reserved_transitions,
                    missing_path_places,
                )
            }
            ProjectedAggregateAssignmentSource::MoveProjection {
                missing_path_places: source_missing_path_places,
                missing_descendant_places,
                ..
            } => projected_subobject_assignment_budget_violation(
                self.budget_values(),
                self.budget_places(),
                self.instructions.len(),
                self.reserved_transitions,
                *source_missing_path_places,
                *missing_descendant_places,
                missing_path_places,
            ),
            ProjectedAggregateAssignmentSource::CloneRoot { .. } => {
                projected_aggregate_clone_assignment_budget_violation(
                    self.budget_values(),
                    self.budget_places(),
                    self.instructions.len(),
                    self.reserved_transitions,
                    self.cleanup_plans.len(),
                    self.cleanup_actions,
                    self.owners.pending().len(),
                    0,
                    missing_path_places,
                )
            }
            ProjectedAggregateAssignmentSource::CloneProjection {
                missing_path_places: source_missing_path_places,
                ..
            } => projected_aggregate_clone_assignment_budget_violation(
                self.budget_values(),
                self.budget_places(),
                self.instructions.len(),
                self.reserved_transitions,
                self.cleanup_plans.len(),
                self.cleanup_actions,
                self.owners.pending().len(),
                *source_missing_path_places,
                missing_path_places,
            ),
        }
    }

    pub(super) fn plan_projected_aggregate_assignment(
        &mut self,
        target: u32,
        value: u32,
        at: Span,
    ) -> Option<ProjectedAggregateAssignmentPlan> {
        if self.projected_aggregate_assignments != 0 {
            self.errors.at(
                "ZRYNA-M3016",
                at,
                "this checkpoint admits only one projected aggregate assignment per function",
                "move one complete static aggregate subobject, or move or clone one complete root, into one static aggregate projection",
            );
            return None;
        }
        let Some(target_preflight) = self.owned_place_preflight(target) else {
            let _ = self.owned_place(target);
            return None;
        };
        let target_ty = target_preflight.place.ty;
        if target_preflight.place.is_root
            || target_ty.is_copy()
            || !matches!(target_ty.category, TypeCategory::Struct | TypeCategory::FixedArray)
            || !self.supported(target_ty)
        {
            self.errors.at(
                "ZRYNA-M3013",
                at,
                "projected aggregate assignment requires one exact non-Copy static Struct or fixed-array target",
                "assign one complete exact aggregate root or distinct static aggregate subobject to a static Struct field or constant fixed-array element",
            );
            return None;
        }
        if !target_preflight.place.mutable
            || !self.preflight_projection_available(&target_preflight)
        {
            self.errors.at(
                "ZRYNA-M3014",
                at,
                "projected aggregate assignment target is immutable, moved, or overlaps a moved subobject",
                "replace one initialized mutable available static aggregate projection",
            );
            return None;
        }
        let source = self.projected_aggregate_assignment_value_source(
            value,
            target_ty,
            target_preflight.place.root,
        )?;
        let clones_projection =
            matches!(&source, ProjectedAggregateAssignmentSource::CloneProjection { .. });
        if clones_projection && !self.projected_aggregate_clone_site_available(at) {
            return None;
        }
        if self.projected_aggregate_assignment_exceeds_budget(&source, target_preflight.missing) {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                "projected aggregate assignment exceeds a checked value, place, transition, or cleanup resource limit",
                "reduce static projection depth, simultaneously live owners, or preceding owned expressions",
            );
            return None;
        }

        Some(ProjectedAggregateAssignmentPlan { target_ty, source, clones_projection })
    }
}
