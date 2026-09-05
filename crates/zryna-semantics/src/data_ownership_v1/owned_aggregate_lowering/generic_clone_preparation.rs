use std::collections::BTreeSet;
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
        ) || matches!(&expression.kind, RawExpressionKind::Reference { name } if self.bindings.get(&name.text).is_some_and(|binding| self.state.parent(binding.place).is_some()))
        {
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
            self.reject_handle_structural_clone(ty, at)?;
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
        self.reject_handle_structural_clone(ty, at)?;
        self.generic_clone_from_place(binding.place, ty, at)
    }

    fn reject_handle_structural_clone(&mut self, ty: Ty, at: Span) -> Option<()> {
        if !contains_handle(ty.layout, self.decisions.layouts) {
            return Some(());
        }
        self.decisions.errors.at(
            "ZRYNA-M3016",
            at,
            "structural clone containing shared or weak handles requires explicit count operations",
            "clone each handle leaf explicitly before rebuilding a static aggregate; dynamic Enum and Vec clone composition is not yet admitted",
        );
        None
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

fn contains_handle(root: zryna_layout::TypeId, layouts: &zryna_layout::VerifiedLayouts) -> bool {
    graph_contains_handle(root, |ty| {
        let record = layouts.type_by_id(ty)?;
        let handle = matches!(
            record.category(),
            zryna_layout::TypeCategory::Shared | zryna_layout::TypeCategory::Weak
        );
        let children = match record.category() {
            zryna_layout::TypeCategory::Struct => {
                record.fields().iter().map(|field| field.ty()).collect()
            }
            zryna_layout::TypeCategory::Enum => {
                record.variants().iter().filter_map(|variant| variant.payload()).collect()
            }
            zryna_layout::TypeCategory::FixedArray | zryna_layout::TypeCategory::Vec => {
                record.referenced_type().into_iter().collect()
            }
            _ => Vec::new(),
        };
        Some((handle, children))
    })
}

fn graph_contains_handle<T: Copy + Ord>(
    root: T,
    mut describe: impl FnMut(T) -> Option<(bool, Vec<T>)>,
) -> bool {
    let mut seen = BTreeSet::new();
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        if !seen.insert(node) {
            continue;
        }
        let Some((handle, children)) = describe(node) else { continue };
        if handle {
            return true;
        }
        pending.extend(children);
    }
    false
}

#[cfg(test)]
#[path = "../tests/handle_reachability.rs"]
mod reachability_tests;

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
