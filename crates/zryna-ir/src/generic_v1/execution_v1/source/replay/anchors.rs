//! Whole source moves rename an existing pending obligation instead of reacquiring it.
use super::{Body, Failure, UntrustedSpan, Value, charge, raw, reject};
use crate::generic_v1::execution_v1::resource_failure;
impl Body<'_, '_> {
    pub(super) fn move_read(
        &mut self,
        place: u32,
        ty: u32,
        span: UntrustedSpan,
    ) -> Result<Value, Failure> {
        let slot = self.pending_anchor(place)?;
        self.consume_place(place, span)?;
        let value = self.emit(
            raw::ValueType::Stored(ty),
            raw::Operation::ReadPlace { place, mode: raw::ReadMode::Move },
            span,
        )?;
        let owner = self.owners[value.id as usize].ok_or(Failure::InternalFailure)?;
        self.restore_anchor(owner, slot)?;
        Ok(value)
    }
    pub(super) fn initialize_from(
        &mut self,
        place: u32,
        value: Value,
        span: UntrustedSpan,
    ) -> Result<(), Failure> {
        let slot =
            self.owners[value.id as usize].map(|owner| self.pending_anchor(owner)).transpose()?;
        self.consume(value, span)?;
        self.instruction(None, raw::Operation::InitializePlace { place, value: value.id }, span)?;
        self.initialize(place)?;
        if let Some(slot) = slot {
            self.restore_anchor(place, slot)?;
        }
        Ok(())
    }
    fn pending_anchor(&mut self, place: u32) -> Result<usize, Failure> {
        charge(
            self.spent,
            self.state
                .order
                .len()
                .checked_add(1)
                .ok_or_else(|| resource_failure("source pending-anchor lookup overflow"))?,
        )?;
        self.state
            .order
            .iter()
            .position(|owner| *owner == place)
            .ok_or_else(|| reject("whole source move lacks its exact pending anchor"))
    }
    fn restore_anchor(&mut self, place: u32, slot: usize) -> Result<(), Failure> {
        // Publication already reserved the appended slot. Pop/insert cannot allocate.
        charge(
            self.spent,
            self.state
                .order
                .len()
                .checked_mul(2)
                .and_then(|n| n.checked_add(1))
                .ok_or_else(|| resource_failure("source pending-anchor restoration overflow"))?,
        )?;
        if self.state.order.last() != Some(&place) || slot >= self.state.order.len() {
            return Err(reject("source publication lost the original pending anchor"));
        }
        let _ = self.state.order.pop();
        self.state.order.insert(slot, place);
        Ok(())
    }
}
