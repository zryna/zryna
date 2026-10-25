//! Exact original-frame Clone capability, independent of closed layout ownership.
use super::{Builder, Closed, Failure, Owned, Value, reject};
use zryna_source::UntrustedSpan;
impl Builder<'_, '_> {
    pub(super) fn clone_struct(
        &mut self,
        value: &Value,
        span: UntrustedSpan,
        keyword: UntrustedSpan,
    ) -> Result<Value, Failure> {
        if !self.type_facts.clone_available(&value.ty, &mut self.budget)? {
            let primary =
                self.sources.verify_span(keyword).map_err(|_| Failure::InternalFailure)?;
            return Err(Failure::Diagnostics(vec![zryna_diagnostics::Diagnostic::error_at(
                "ZRYNA-M7002",
                primary,
                "original stored members do not provide Clone",
                "clone only a value whose original stored members provide Clone",
            )]));
        }
        if !matches!(&value.ty, Closed::Stored(key) if matches!(key.first(), Some(0x10 | 0x12))) {
            return Err(reject("this Clone operation requires an exact stored Struct source"));
        }
        self.type_facts.key_copy(&value.ty, 1, &mut self.budget)?;
        self.ext(value.ty.clone(), span, Owned::CloneStruct { value: value.id })
    }
}
