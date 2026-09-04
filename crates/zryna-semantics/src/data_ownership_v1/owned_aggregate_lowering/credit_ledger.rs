use super::{ConstructorCommitReservation, ConstructorStorage};

// Live and scratch accounting borrow the same credit operations without moving storage.
pub(in crate::data_ownership_v1::owned_aggregate_lowering) struct CreditLedgerMut<'a> {
    pub(in crate::data_ownership_v1::owned_aggregate_lowering) storage: &'a mut ConstructorStorage,
    pub(in crate::data_ownership_v1::owned_aggregate_lowering) transitions: &'a mut usize,
}

impl CreditLedgerMut<'_> {
    pub(in crate::data_ownership_v1::owned_aggregate_lowering) fn acquire_constructor(
        &mut self,
        arity: usize,
        places: usize,
    ) -> Option<ConstructorCommitReservation> {
        let next_operands = self.storage.operands.checked_add(arity)?;
        let next_transitions = self.transitions.checked_add(1)?;
        let next_values = self.storage.values.checked_add(1)?;
        let next_places = self.storage.places.checked_add(places)?;
        self.storage.operands = next_operands;
        *self.transitions = next_transitions;
        self.storage.values = next_values;
        self.storage.places = next_places;
        Some(ConstructorCommitReservation {
            storage: ConstructorStorage { values: 1, places, operands: arity },
        })
    }

    pub(in crate::data_ownership_v1::owned_aggregate_lowering) fn release_constructor(
        &mut self,
        ticket: ConstructorCommitReservation,
    ) {
        let ConstructorCommitReservation { storage } = ticket;
        let places =
            self.storage.places.checked_sub(storage.places).expect("held constructor place credit");
        let values =
            self.storage.values.checked_sub(storage.values).expect("held constructor value credit");
        let transitions =
            self.transitions.checked_sub(1).expect("held constructor transition credit");
        let operands = self
            .storage
            .operands
            .checked_sub(storage.operands)
            .expect("held constructor operand credit");
        self.storage.places = places;
        self.storage.values = values;
        *self.transitions = transitions;
        self.storage.operands = operands;
    }

    pub(in crate::data_ownership_v1::owned_aggregate_lowering) fn acquire_assignment(&mut self) {
        *self.transitions =
            self.transitions.checked_add(1).expect("assignment transition capacity preflighted");
    }

    pub(in crate::data_ownership_v1::owned_aggregate_lowering) fn release_assignment(&mut self) {
        *self.transitions =
            self.transitions.checked_sub(1).expect("reserved aggregate assignment transition");
    }
}
