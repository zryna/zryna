#[cfg(test)]
use zryna_ir::data_ownership_v1 as ir;
use zryna_source::Span;

use super::super::type_model::Ty;
use super::PrivateOwnedAggregateLowerer;
use super::resource_decisions::AggregateUsage;

#[path = "credit_ledger.rs"]
mod credit_ledger;
pub(super) use credit_ledger::CreditLedgerMut;

#[cfg(test)]
#[path = "../tests/aggregate_constructor_envelope.rs"]
pub(super) mod tests;

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct ConstructorStorage {
    values: usize,
    places: usize,
    operands: usize,
}

impl ConstructorStorage {
    pub(super) fn counts(&self) -> [usize; 3] {
        [self.values, self.places, self.operands]
    }
}

// This affine ticket owns only the final constructor's capacity, not its child effects.
pub(super) struct ConstructorCommitReservation {
    storage: ConstructorStorage,
}

impl ConstructorCommitReservation {
    pub(super) fn release(self, lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>) {
        lowerer.credit_ledger().release_constructor(self);
    }
}

impl PrivateOwnedAggregateLowerer<'_, '_, '_> {
    pub(super) fn preparation_storage(&self) -> ConstructorStorage {
        ConstructorStorage {
            values: self.constructor_storage.values,
            places: self.constructor_storage.places,
            operands: self.constructor_storage.operands,
        }
    }

    pub(super) fn credit_ledger(&mut self) -> CreditLedgerMut<'_> {
        CreditLedgerMut {
            storage: &mut self.constructor_storage,
            transitions: &mut self.reserved_transitions,
        }
    }

    pub(super) fn resource_usage(&self) -> AggregateUsage {
        AggregateUsage {
            values: self.next_value as usize,
            places: self.places.len(),
            transitions: self.instructions.len(),
            operands: self.aggregate_operands,
            held_values: self.constructor_storage.values,
            held_places: self.constructor_storage.places,
            held_transitions: self.reserved_transitions,
            held_operands: self.constructor_storage.operands,
        }
    }

    // Overflow is represented only as an over-limit preflight input. These counts never
    // determine emitted identities or become committed arena/accounting state.
    pub(super) fn budget_values(&self) -> usize {
        (self.next_value as usize).saturating_add(self.constructor_storage.values)
    }

    pub(super) fn budget_places(&self) -> usize {
        self.places.len().saturating_add(self.constructor_storage.places)
    }

    pub(super) fn reserved_constructor_places(&self) -> usize {
        self.constructor_storage.places
    }

    #[cfg(test)]
    pub(super) fn budget_operands(&self) -> usize {
        self.aggregate_operands.saturating_add(self.constructor_storage.operands)
    }

    #[cfg(test)]
    pub(super) fn budget_transitions(&self) -> usize {
        self.instructions.len().saturating_add(self.reserved_transitions)
    }

    pub(super) fn constructor_storage_is_clear(&self) -> bool {
        self.constructor_storage == ConstructorStorage::default()
    }

    pub(super) fn preflight_constructor_operands(&mut self, additional: usize, at: Span) -> bool {
        self.resource_usage().operands(additional, at, self.errors)
    }

    #[cfg(test)]
    pub(super) fn preflight_value(&mut self, at: Span) -> bool {
        self.resource_usage().value(at, self.errors)
    }

    #[cfg(test)]
    pub(super) fn preflight_constructor_places(&mut self, additional: usize, at: Span) -> bool {
        self.resource_usage().places(additional, at, self.errors)
    }

    pub(super) fn reserve_constructor_commit(
        &mut self,
        result: Ty,
        arity: usize,
        at: Span,
    ) -> Option<ConstructorCommitReservation> {
        if !self.resource_usage().constructor(result, arity, at, self.errors) {
            return None;
        }
        self.credit_ledger().acquire_constructor(arity, usize::from(!result.is_copy()))
    }
}
