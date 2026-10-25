//! Exact inherited concat preparation and immediate reverse temporary observation cleanup.
use super::{Body, Failure, UntrustedSpan, Value, charge, raw, reject};
impl Body<'_, '_> {
    pub(super) fn concat(
        &mut self,
        arguments: &[u32],
        has_types: bool,
        keyword: UntrustedSpan,
        span: UntrustedSpan,
        depth: usize,
    ) -> Result<Value, Failure> {
        let [left, right] = arguments else {
            return Err(self.error(
                keyword,
                "ZRYNA-M7006",
                "concat requires exactly two String operands",
            ));
        };
        if has_types {
            return Err(reject("concat does not accept type arguments"));
        }
        charge(self.spent, arguments.len() + 1)?;
        let mark = self.output.loans.len();
        let ty = self.primitive(2)?;
        let (left, left_temporary) = self.observed(*left, span, depth)?;
        if raw::ValueType::Stored(left.ty) != ty {
            return Err(reject("concat left operand is not String"));
        }
        self.available_target(&left, span)?;
        let (right, right_temporary) = self.observed(*right, span, depth)?;
        if raw::ValueType::Stored(right.ty) != ty {
            return Err(reject("concat right operand is not String"));
        }
        self.available_target(&left, span)?;
        self.available_target(&right, span)?;
        let failure = self.failure(span)?;
        let value = self.emit(
            ty,
            raw::Operation::StringConcat { left: left.source, right: right.source, failure },
            span,
        )?;
        self.end_created_loans(mark, span)?;
        if right_temporary {
            self.drop_observed(&right, span)?;
        }
        if left_temporary {
            self.drop_observed(&left, span)?;
        }
        Ok(value)
    }
}
