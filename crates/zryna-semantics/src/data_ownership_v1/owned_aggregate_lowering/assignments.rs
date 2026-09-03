use zryna_ir::data_ownership_v1::raw;
use zryna_source::Span;

use super::super::type_model::{ProjectedAggregateAssignmentSource, ProjectedAggregateMoveContext};
use super::PrivateOwnedAggregateLowerer;

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn lower_projected_aggregate_assignment(
        &mut self,
        target: u32,
        value: u32,
        at: Span,
    ) -> Option<()> {
        let plan = self.plan_projected_aggregate_assignment(target, value, at)?;
        let target_ty = plan.target_ty;
        let source = plan.source;
        let clones_projection = plan.clones_projection;

        let target = self.owned_place(target)?;
        debug_assert_eq!(target.ty, target_ty);
        debug_assert!(!target.is_root);
        if !self.reserve_transition(at) {
            return None;
        }
        let prepared = match &source {
            ProjectedAggregateAssignmentSource::MoveRoot { name, at } => {
                self.reference_value(name, target_ty, *at)
            }
            ProjectedAggregateAssignmentSource::MoveProjection { expression, .. } => self
                .projected_value(
                    *expression,
                    target_ty,
                    Some(ProjectedAggregateMoveContext::ProjectedReplacement),
                ),
            ProjectedAggregateAssignmentSource::CloneRoot { binding, at } => {
                self.emit_aggregate_clone(binding, target_ty, *at)
            }
            ProjectedAggregateAssignmentSource::CloneProjection { expression, at, .. } => {
                self.emit_projected_aggregate_clone(*expression, target_ty, *at)
            }
        };
        self.release_transition();
        let prepared = prepared?;
        if !self.emit_effect(
            at,
            raw::InstructionKind::ReplacePlace { place: target.place, value: prepared },
        ) {
            return None;
        }
        if self.owners.transfer(prepared).is_none() {
            self.errors.at(
                "ZRYNA-M3014",
                at,
                "projected aggregate assignment has no distinct prepared owner",
                "move one available static aggregate subobject, or move or clone one independently owned root, into the projection",
            );
            return None;
        }
        self.projected_aggregate_assignments += 1;
        if clones_projection {
            self.projected_aggregate_clones += 1;
        }
        Some(())
    }
}
