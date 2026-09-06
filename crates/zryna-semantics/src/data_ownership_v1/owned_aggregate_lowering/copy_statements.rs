use zryna_ir::data_ownership_v1::raw;
use zryna_syntax::v4::RawExpressionKind;

use super::super::diagnostics::span;
use super::PrivateOwnedAggregateLowerer;

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn lower_copy_assignment(&mut self, target: u32, value: u32) -> Option<bool> {
        let target_expression = self.expression(target)?.clone();
        let RawExpressionKind::Reference { name } = target_expression.kind else {
            return Some(false);
        };
        let Some(binding) = self.bindings.get(&name.text).cloned() else {
            return Some(false);
        };
        if !binding.ty.is_copy() {
            return Some(false);
        }
        let target_at = span(self.input.sources(), name.span);
        if !binding.mutable {
            self.errors.at(
                "ZRYNA-M3013",
                target_at,
                format!("binding '{}' is not mutable", name.text),
                "assign only to a mutable let binding",
            );
            return None;
        }
        let prepared = self.value(value, binding.ty)?;
        if !self.emit_effect(
            span(self.input.sources(), target_expression.span),
            raw::InstructionKind::ReplacePlace { place: binding.place, value: prepared },
        ) {
            return None;
        }
        Some(true)
    }
}
