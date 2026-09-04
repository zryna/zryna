use zryna_ir::data_ownership_v1 as ir;
use zryna_source::Span;

use super::super::global_resource_limits::{
    aggregate_operand_budget_violation, resource_budget_violation,
};
use super::super::type_model::Ty;
use super::PrivateOwnedAggregateLowerer;

#[cfg(test)]
#[path = "../tests/aggregate_constructor_envelope.rs"]
mod tests;

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct ConstructorStorage {
    values: usize,
    places: usize,
    operands: usize,
}

// This affine ticket owns only the final constructor's capacity, not its child effects.
pub(super) struct ConstructorCommitReservation {
    storage: ConstructorStorage,
}

impl ConstructorCommitReservation {
    pub(super) fn release(self, lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>) {
        let Self { storage } = self;
        let places = lowerer
            .constructor_storage
            .places
            .checked_sub(storage.places)
            .expect("held constructor place credit");
        let values = lowerer
            .constructor_storage
            .values
            .checked_sub(storage.values)
            .expect("held constructor value credit");
        let transitions = lowerer
            .reserved_transitions
            .checked_sub(1)
            .expect("held constructor transition credit");
        let operands = lowerer
            .constructor_storage
            .operands
            .checked_sub(storage.operands)
            .expect("held constructor operand credit");
        lowerer.constructor_storage.places = places;
        lowerer.constructor_storage.values = values;
        lowerer.reserved_transitions = transitions;
        lowerer.constructor_storage.operands = operands;
    }
}

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    // Overflow is represented only as an over-limit preflight input. These counts never
    // determine emitted identities or become committed arena/accounting state.
    pub(super) fn budget_values(&self) -> usize {
        (self.next_value as usize).saturating_add(self.constructor_storage.values)
    }

    pub(super) fn budget_places(&self) -> usize {
        self.places.len().saturating_add(self.constructor_storage.places)
    }

    pub(super) fn budget_operands(&self) -> usize {
        self.aggregate_operands.saturating_add(self.constructor_storage.operands)
    }

    pub(super) fn budget_transitions(&self) -> usize {
        self.instructions.len().saturating_add(self.reserved_transitions)
    }

    pub(super) fn constructor_storage_is_clear(&self) -> bool {
        self.constructor_storage == ConstructorStorage::default()
    }

    pub(super) fn preflight_constructor_operands(&mut self, additional: usize, at: Span) -> bool {
        if aggregate_operand_budget_violation(self.budget_operands(), additional) {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                format!(
                    "derived aggregate operands exceed the M3 limit of {}",
                    ir::MAX_AGGREGATE_OPERANDS
                ),
                "reduce Struct fields and fixed-array elements",
            );
            return false;
        }
        true
    }

    pub(super) fn preflight_value(&mut self, at: Span) -> bool {
        if self.budget_values() >= ir::MAX_VALUES_PER_FUNCTION {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                format!(
                    "derived values exceed the per-function M3 limit of {}",
                    ir::MAX_VALUES_PER_FUNCTION
                ),
                "reduce private aggregate expressions",
            );
            return false;
        }
        true
    }

    pub(super) fn preflight_constructor_places(&mut self, additional: usize, at: Span) -> bool {
        if resource_budget_violation(self.budget_places(), additional, ir::MAX_PLACES_PER_FUNCTION)
        {
            self.errors.at(
                "ZRYNA-M3201",
                at,
                format!(
                    "derived places exceed the per-function M3 limit of {}",
                    ir::MAX_PLACES_PER_FUNCTION
                ),
                "reduce owned aggregate temporaries and locals",
            );
            return false;
        }
        true
    }

    pub(super) fn reserve_constructor_commit(
        &mut self,
        result: Ty,
        arity: usize,
        at: Span,
    ) -> Option<ConstructorCommitReservation> {
        if !self.preflight_constructor_operands(arity, at)
            || !self.preflight_transition(1, at)
            || !self.preflight_value(at)
            || !self.preflight_constructor_places(usize::from(!result.is_copy()), at)
        {
            return None;
        }
        let places = usize::from(!result.is_copy());
        let next_operands = self.constructor_storage.operands.checked_add(arity)?;
        let next_transitions = self.reserved_transitions.checked_add(1)?;
        let next_values = self.constructor_storage.values.checked_add(1)?;
        let next_places = self.constructor_storage.places.checked_add(places)?;
        self.constructor_storage.operands = next_operands;
        self.reserved_transitions = next_transitions;
        self.constructor_storage.values = next_values;
        self.constructor_storage.places = next_places;
        Some(ConstructorCommitReservation {
            storage: ConstructorStorage { values: 1, places, operands: arity },
        })
    }
}
