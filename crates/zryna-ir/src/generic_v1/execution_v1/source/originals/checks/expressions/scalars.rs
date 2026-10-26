//! Exact original scalar roles: opaque declared parameters cannot gain scalar capabilities.
use super::{Checker, Closed, Failure, UntrustedSpan, Use, charge, key, scalar};
impl Checker<'_, '_, '_> {
    fn scalar_operand(
        &mut self,
        operand: u32,
        operator: UntrustedSpan,
        depth: usize,
    ) -> Result<Closed, Failure> {
        let value = self.expression(operand, Use::Move, depth + 1)?;
        if let Closed::Stored(bytes) = &value {
            // This existing bounded reader prepays exact key bytes/children and validates
            // the complete direct head, including the real owner-frame opaque slot tags.
            let shape = key::shape(bytes, self.spent)?;
            if matches!(shape.tag, 0x30 | 0x31) {
                return Err(self.error(
                    operator,
                    "ZRYNA-M7002",
                    "opaque ZrynaValue parameter does not provide scalar arithmetic or comparison",
                ));
            }
        }
        Ok(value)
    }
    pub(super) fn original_negation(
        &mut self,
        operand: u32,
        span: UntrustedSpan,
        operator: UntrustedSpan,
        depth: usize,
    ) -> Result<Closed, Failure> {
        let value = self.scalar_operand(operand, operator, depth)?;
        let expected = scalar(1, self.spent)?;
        self.expect(&value, &expected, span)?;
        Ok(expected)
    }
    pub(super) fn original_integer_binary(
        &mut self,
        lhs: u32,
        rhs: u32,
        arithmetic: bool,
        span: UntrustedSpan,
        operator: UntrustedSpan,
        depth: usize,
    ) -> Result<Closed, Failure> {
        let left = self.scalar_operand(lhs, operator, depth)?;
        let right = self.scalar_operand(rhs, operator, depth)?;
        let expected = scalar(1, self.spent)?;
        self.expect(&left, &expected, span)?;
        self.expect(&right, &expected, span)?;
        scalar(u8::from(arithmetic), self.spent)
    }
    pub(super) fn original_equality(
        &mut self,
        lhs: u32,
        rhs: u32,
        span: UntrustedSpan,
        operator: UntrustedSpan,
        depth: usize,
    ) -> Result<Closed, Failure> {
        let left = self.scalar_operand(lhs, operator, depth)?;
        let right = self.scalar_operand(rhs, operator, depth)?;
        self.expect(&left, &right, span)?;
        charge(self.spent, 1)?;
        if !matches!(&left, Closed::Stored(key) if matches!(key.as_slice(), [0 | 1])) {
            return Err(self.error(
                span,
                "ZRYNA-M7006",
                "original equality requires a scalar type",
            ));
        }
        scalar(0, self.spent)
    }
}
