use zryna_ir::data_ownership_v1::raw;
use zryna_layout::TypeCategory;
use zryna_source::Span;
use zryna_syntax::v4::RawExpressionKind;

use super::super::Ty;
use super::super::owned_lowering_resources::CleanupRecipe;
use super::preparation_operations::PreparationContext;
use super::preparation_plan::{Leaf, Operation};

pub(super) fn checked_index(kind: &RawExpressionKind, length: u64) -> bool {
    !matches!(kind, RawExpressionKind::I32Literal { spelling }
        if spelling.parse::<i32>().ok().and_then(|index| u64::try_from(index).ok())
            .is_some_and(|index| index < length))
}

impl super::PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn is_checked_array_index(&self, id: u32) -> bool {
        let Some(expression) = self.expression(id) else { return false };
        let RawExpressionKind::Index { base, index, .. } = expression.kind else { return false };
        let Some(ty) = self.indexed_expression_type(base) else { return false };
        ty.category == TypeCategory::FixedArray
            && self
                .layouts
                .type_by_id(ty.layout)
                .and_then(zryna_layout::VerifiedType::array_length)
                .zip(self.expression(index))
                .is_some_and(|(length, index)| {
                    self.projection_expression_type(base).is_none()
                        || checked_index(&index.kind, length)
                })
    }
}

impl PreparationContext<'_, '_, '_, '_> {
    pub(super) fn clone_indexed_borrow(
        &mut self,
        borrow: raw::BorrowId,
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
        self.emit_leaf(Leaf::IndexedClone { borrow, cleanup, prefix }, ty, at)
    }
}
