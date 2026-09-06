use super::super::diagnostics::span;
use super::indexed_vec_preparation::IndexedObservation;
use super::preparation_operations::PreparationContext;
use super::preparation_plan::Operation;
use super::{PrivateOwnedAggregateLowerer, Ty};
use zryna_ir::data_ownership_v1::raw;

impl PreparationContext<'_, '_, '_, '_> {
    pub(super) fn structured_copy(
        &mut self,
        id: u32,
        expected: Option<Ty>,
    ) -> Option<IndexedObservation> {
        let Some((value, ty)) = self.state.facts.structured_values.remove(&id) else {
            return Some(IndexedObservation::Unselected);
        };
        let at = span(
            self.decisions.input.sources(),
            self.decisions.function.body.expressions.get(id as usize)?.span,
        );
        if !ty.is_copy()
            || expected.is_some_and(|expected| expected != ty)
            || self.state.types.as_ref().ok()?.get(value) != Some(ty.ir)
        {
            self.decisions.errors.at(
                "ZRYNA-M3016",
                at,
                "structured operand does not have its exact existing Copy value",
                "resume only the once-evaluated exact Copy operand",
            );
            return None;
        }
        self.push(Operation::StructuredCopy { expression: id, value }, ty, at, Some(value));
        Some(IndexedObservation::Value(value))
    }
}

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn consume_structured_copy(
        &mut self,
        expression: u32,
        value: raw::ValueId,
        ty: Ty,
    ) -> super::state::Emission {
        assert!(ty.is_copy(), "structured handoff is Copy-only");
        assert_eq!(
            self.preparation_facts.structured_values.remove(&expression),
            Some((value, ty)),
            "exact once-evaluated structured operand"
        );
        assert_eq!(
            self.constructor_types
                .observed_snapshot(&self.instructions)
                .expect("structured Copy type authority")
                .get(value),
            Some(ty.ir)
        );
        super::state::Emission { value, owners: Vec::new() }
    }
}
