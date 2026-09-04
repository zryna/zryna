use super::super::super::super::scalar_operations::ScalarOperation;
use super::{Consumption, Span, Ty, raw};
use zryna_layout::TypeCategory;

struct OpenScalar {
    start: usize,
    end: usize,
    depth: usize,
    ty: Ty,
    kind: ScalarOperation,
    expected: Vec<(raw::ValueId, Ty)>,
    values: Vec<(raw::ValueId, Ty)>,
}

#[derive(Default)]
pub(super) struct ScalarScopes {
    open: Vec<OpenScalar>,
}

impl ScalarScopes {
    pub(super) fn start(&self) -> Option<usize> {
        self.open.last().map(|scope| scope.start)
    }
    pub(super) fn enter(
        &mut self,
        (start, end, length): (usize, usize, usize),
        depth: usize,
        ty: Ty,
        kind: ScalarOperation,
        expected: Vec<(raw::ValueId, Ty)>,
    ) {
        assert!(end > start + 1 && end <= length, "scalar scope range");
        assert_eq!(
            expected.len(),
            if kind == ScalarOperation::Neg { 1 } else { 2 },
            "scalar scope arity"
        );
        if let Some(parent) = self.open.last() {
            assert!(end < parent.end, "nested scalar scope range");
        }
        self.open.push(OpenScalar { start, end, depth, ty, kind, expected, values: Vec::new() });
    }
    pub(super) fn result(&mut self, value: raw::ValueId, ty: Ty, depth: usize) -> bool {
        if let Some(scope) = self.open.last_mut()
            && scope.depth == depth
        {
            assert_eq!(
                scope.expected.get(scope.values.len()),
                Some(&(value, ty)),
                "scalar ordered immediate operand"
            );
            scope.values.push((value, ty));
            return false;
        }
        true
    }
    pub(super) fn complete(&self) -> bool {
        self.open.is_empty()
    }
}

impl Consumption<'_, '_, '_, '_> {
    pub(super) fn scalar_commit(
        &mut self,
        index: usize,
        ty: Ty,
        at: Span,
        kind: ScalarOperation,
        operands: &[(raw::ValueId, Ty)],
    ) -> super::super::super::state::Emission {
        assert!(self.cleanups.is_empty(), "scalar commit cannot consume cleanup");
        let scope = self.scalars.open.pop().expect("scalar commit owns scope");
        assert_eq!(
            (scope.end, scope.depth, scope.ty, scope.kind),
            (index + 1, self.open.len(), ty, kind),
            "scalar exact scope contract"
        );
        assert_eq!(scope.values, operands, "scalar complete ordered operands");
        assert_eq!(scope.expected, operands, "scalar entry commit linkage");
        let lhs = operands[0];
        let rhs = operands.get(1).copied();
        let integer = self
            .lowerer
            .node_types
            .iter()
            .flatten()
            .find(|ty| ty.category == TypeCategory::I32)
            .copied();
        kind.validate(integer, lhs.1, rhs.map(|(_, ty)| ty), at, self.lowerer.errors)
            .expect("scalar actual input types");
        let category = kind.result_category();
        let actual = self
            .lowerer
            .node_types
            .iter()
            .flatten()
            .find(|ty| ty.category == category)
            .copied()
            .expect("scalar primitive result");
        assert_eq!(ty, actual, "scalar exact result type");
        self.lowerer
            .emit_recorded(ty, at, kind.instruction(lhs.0, rhs.map(|(value, _)| value)))
            .expect("prepared scalar emission")
    }
}
