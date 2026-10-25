//! Source append establishes target before RHS and retains every pending owner on failure.
use super::{Body, Failure, Kind, UntrustedSpan, Value, charge, raw};
impl Body<'_, '_> {
    pub(super) fn push_vector(
        &mut self,
        vector: u32,
        rhs: u32,
        span: UntrustedSpan,
        keyword: UntrustedSpan,
        depth: usize,
    ) -> Result<Value, Failure> {
        let mark = self.output.loans.len();
        let target = self.source_place_mode(
            vector,
            depth + 1,
            Some(span),
            (raw::Access::Exclusive, raw::RegionRole::TransientIndexedAccess),
        )?;
        self.assignment_loan_roles(mark)?;
        charge(self.spent, 2)?;
        if self.world.kind(target.ty) != Some(Kind::Vec) {
            return Err(self.error(keyword, "ZRYNA-M7006", "push requires exact original Vec"));
        }
        if !target.mutable {
            return Err(self.error(keyword, "ZRYNA-M7006", "push requires mutable source binding"));
        }
        self.available_target(&target, span)?;
        let element = self
            .world
            .arguments(target.ty)
            .and_then(|a| a.first())
            .copied()
            .ok_or(Failure::InternalFailure)?;
        let value = self.expression(rhs, depth + 1)?;
        if value.ty != raw::ValueType::Stored(element) {
            return Err(self.error(
                self.original.body.expressions[rhs as usize].span,
                "ZRYNA-M7006",
                "push RHS differs from exact original element",
            ));
        }
        // The very same retained target identity is checked after RHS effects.
        self.available_target(&target, span)?;
        let failure = self.failure(span)?;
        self.consume(value, span)?;
        self.available_target(&target, span)?;
        self.instruction(
            None,
            raw::Operation::VecPush { vector: target.source, value: value.id, failure },
            span,
        )?;
        self.end_created_loans(mark, span)?;
        self.emit(raw::ValueType::Unit, raw::Operation::Unit, span)
    }
}
