use zryna_ir::data_ownership_v1::raw;
use zryna_source::Span;
use zryna_syntax::v4::RawExpressionKind;

use super::super::Ty;
use super::super::diagnostics::span;
use super::super::owned_lowering_resources::CleanupRecipe;
use super::PrivateOwnedAggregateLowerer;
use super::availability::AvailabilityView;
use super::preparation_operations::PreparationContext;
use super::preparation_plan::{Leaf, Operation};

impl PreparationContext<'_, '_, '_, '_> {
    pub(super) fn generic_clone(&mut self, id: u32, ty: Ty, at: Span) -> Option<raw::ValueId> {
        let expression = self.decisions.function.body.expressions.get(id as usize)?;
        if matches!(
            expression.kind,
            RawExpressionKind::FieldAccess { .. } | RawExpressionKind::Index { .. }
        ) {
            let source = self.resolve(id)?;
            let state = &self.state;
            let available =
                AvailabilityView::new(&state.owners, &state.moved, &state.partial, |id| {
                    state.parent(id)
                });
            if source.ty != ty
                || ty.is_copy()
                || !available.projection_available(source.place, source.root)
            {
                self.decisions.errors.at(
                    "ZRYNA-M3014",
                    at,
                    "structural clone requires one complete available exact static subobject",
                    "clone the exact initialized subobject before moving it or any descendant",
                );
                return None;
            }
            return self.generic_clone_from_place(source.place, ty, at);
        }
        let RawExpressionKind::Reference { name } = &expression.kind else {
            super::clone_decisions::nonaddressable_clone(
                span(self.decisions.input.sources(), expression.span),
                self.decisions.errors,
            );
            return None;
        };
        let Some(binding) = self.bindings.get(&name.text) else {
            self.decisions.errors.at(
                "ZRYNA-M3002",
                span(self.decisions.input.sources(), name.span),
                format!("aggregate binding '{}' is not declared in this function", name.text),
                "clone one preceding available aggregate local",
            );
            return None;
        };
        if binding.ty != ty || ty.is_copy() {
            self.decisions.errors.at(
                "ZRYNA-M3016",
                span(self.decisions.input.sources(), name.span),
                "structural clone source has the wrong exact aggregate type",
                "clone a local with the exact contextual aggregate type",
            );
            return None;
        }
        let state = &self.state;
        if !AvailabilityView::new(&state.owners, &state.moved, &state.partial, |id| {
            state.parent(id)
        })
        .whole_root_available(binding.place)
        {
            self.decisions.errors.at(
                "ZRYNA-M3014",
                span(self.decisions.input.sources(), name.span),
                format!("aggregate value '{}' is moved or only partially available", name.text),
                "clone the aggregate only before moving any owned projection",
            );
            return None;
        }
        self.generic_clone_from_place(binding.place, ty, at)
    }

    fn generic_clone_from_place(
        &mut self,
        source: raw::PlaceId,
        ty: Ty,
        at: Span,
    ) -> Option<raw::ValueId> {
        self.push(Operation::CloneCapacity { aggregate: true }, ty, at, None);
        let cleanup = self.reverse(ty, at)?;
        let owner = raw::PlaceId(u32::try_from(self.state.counts[1]).ok()?);
        let recipe = CleanupRecipe::generic_clone_prefix(
            self.state.counts[4],
            self.state.owners.pending(),
            owner,
        )?;
        let (prefix, actions) = (recipe.id, recipe.action_count);
        self.state.counts[4] = self.state.counts[4].checked_add(1)?;
        self.state.counts[5] = self.state.counts[5].checked_add(actions)?;
        self.push(Operation::GenericClonePrefix { id: prefix, owner, actions }, ty, at, None);
        self.emit_leaf(Leaf::GenericClone { source, cleanup, prefix }, ty, at)
    }
}

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn consume_generic_clone_prefix(
        &mut self,
        id: raw::CleanupPlanId,
        owner: raw::PlaceId,
        actions: usize,
        at: Span,
    ) {
        assert_eq!(owner.0 as usize, self.places.len(), "generic clone pending result owner");
        let recipe = CleanupRecipe::generic_clone_prefix(
            self.cleanup_plans.len(),
            self.owners.pending(),
            owner,
        )
        .expect("prepared generic clone prefix recipe");
        assert_eq!(recipe.id, id, "generic clone prefix identity");
        assert_eq!(recipe.action_count, actions, "generic clone prefix action count");
        let actions = recipe.into_actions().collect();
        self.cleanup_plans.push(raw::CleanupPlan { id, span: at, actions });
        self.cleanup_actions = self
            .cleanup_actions
            .checked_add(self.cleanup_plans.last().expect("prefix cleanup").actions.len())
            .expect("prepared generic clone cleanup accounting");
    }
}
