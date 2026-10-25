//! Getter-only loan/projection lookup; metadata never grants a source capability.
use super::{Diagnostic, Model, invariant};
use zryna_ir::generic_v1::execution_v1::{VerifiedLoan, raw};
impl<'p, 'a> Model<'p, 'a, '_> {
    pub(in crate::generic_execution_v1) fn loan(
        &self,
        function: usize,
        id: u32,
    ) -> Result<VerifiedLoan<'p, 'a>, Diagnostic> {
        self.take(id as usize + 1)?;
        self.functions[function].loans().nth(id as usize).ok_or_else(invariant)
    }
    pub(in crate::generic_execution_v1) fn loan_offset(
        &self,
        function: usize,
        id: u32,
    ) -> Result<u32, Diagnostic> {
        self.take(1)?;
        Ok(self.memory.claims().functions[function]
            .loans
            .get(id as usize)
            .ok_or_else(invariant)?
            .offset)
    }
    pub(in crate::generic_execution_v1) fn projection(
        &self,
        ty: u32,
        projection: raw::StaticProjection,
    ) -> Result<u32, Diagnostic> {
        self.take(1)?;
        let ty = self.types.get(ty as usize).ok_or_else(invariant)?;
        let offset = match projection {
            raw::StaticProjection::StructField(ordinal) => {
                self.take(ordinal as usize + 1)?;
                ty.fields().nth(ordinal as usize).ok_or_else(invariant)?.2
            }
            raw::StaticProjection::EnumPayload(_) => ty.payload().ok_or_else(invariant)?.0,
            raw::StaticProjection::FixedArrayConstant(index) => ty
                .array_stride()
                .ok_or_else(invariant)?
                .checked_mul(u64::from(index))
                .ok_or_else(super::budget)?,
        };
        crate::generic_execution_v1::checked_u32(offset)
    }
}
