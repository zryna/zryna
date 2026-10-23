//! Exact frozen `loan_operations` field order and explicit closed tags.
use super::{Failure, invalid, raw, writer::Writer};
impl Writer {
    pub(super) fn operation_loan(&mut self, value: &raw::Operation) -> Result<(), Failure> {
        match value {
            raw::Operation::BeginLoan { loan, failure } => {
                self.byte(0x40)?;
                self.u32(*loan)?;
                self.optional_id(*failure)?;
                Ok(())
            }
            raw::Operation::EndLoan { loan } => {
                self.byte(0x41)?;
                self.u32(*loan)?;
                Ok(())
            }
            raw::Operation::ReadLoan { loan } => {
                self.byte(0x42)?;
                self.u32(*loan)?;
                Ok(())
            }
            raw::Operation::ReplaceLoan { loan, value } => {
                self.byte(0x43)?;
                self.u32(*loan)?;
                self.u32(*value)?;
                Ok(())
            }
            raw::Operation::IndexCopy { base, index, failure } => {
                self.byte(0x44)?;
                self.place_or_loan(*base)?;
                self.u32(*index)?;
                self.u32(*failure)?;
                Ok(())
            }
            _ => Err(invalid()),
        }
    }
}
