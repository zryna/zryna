use super::super::{RawExpressionKind, Span, Ty, raw};
use super::{BorrowBinding, FunctionLowerer};

impl FunctionLowerer<'_, '_, '_> {
    pub(in crate::data_ownership_v1) fn binding_name_exists(&self, candidate: &str) -> bool {
        self.bindings
            .keys()
            .chain(self.borrow_bindings.keys())
            .any(|name| name.eq_ignore_ascii_case(candidate))
    }

    pub(in crate::data_ownership_v1) fn borrow_reference(&self, id: u32) -> Option<BorrowBinding> {
        let expression =
            usize::try_from(id).ok().and_then(|index| self.function.body.expressions.get(index))?;
        let RawExpressionKind::Reference { name } = &expression.kind else {
            return None;
        };
        self.borrow_bindings.get(&name.text).copied()
    }

    pub(in crate::data_ownership_v1) fn push_place(
        &mut self,
        ty: Ty,
        span: Span,
        kind: raw::PlaceKind,
    ) -> raw::PlaceId {
        let id = raw::PlaceId(u32::try_from(self.places.len()).unwrap_or(u32::MAX));
        self.places.push(raw::Place { id, ty: ty.ir, span, kind });
        id
    }
    pub(in crate::data_ownership_v1) fn emit(
        &mut self,
        result_ty: Option<Ty>,
        span: Span,
        kind: raw::InstructionKind,
    ) -> Option<raw::ValueId> {
        let result = result_ty.map(|ty| {
            let id = raw::ValueId(self.values);
            self.values += 1;
            raw::ValueDefinition { id, ty: ty.ir, span }
        });
        let id = result.map(|v| v.id);
        self.instructions.push(raw::Instruction { result, span, kind });
        id
    }
    pub(in crate::data_ownership_v1) fn require_type(
        &mut self,
        expected: Ty,
        actual: Ty,
        at: Span,
        what: &str,
    ) -> Option<()> {
        super::super::scalar_operations::require_type(expected, actual, at, what, self.errors)
    }
}
