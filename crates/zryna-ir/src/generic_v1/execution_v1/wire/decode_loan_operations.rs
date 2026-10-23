//! Exact frozen `loan_operations` field order and explicit closed tags.
use super::{Failure, invalid, raw, reader::Reader};
impl Reader<'_> {
    pub(super) fn operation_loan(&mut self, tag: u8) -> Result<raw::Operation, Failure> {
        match tag {
            0x40 => {
                let loan = self.u32()?;
                let failure = self.option(Self::u32)?;
                Ok(raw::Operation::BeginLoan { loan, failure })
            }
            0x41 => {
                let loan = self.u32()?;
                Ok(raw::Operation::EndLoan { loan })
            }
            0x42 => {
                let loan = self.u32()?;
                Ok(raw::Operation::ReadLoan { loan })
            }
            0x43 => {
                let loan = self.u32()?;
                let value = self.u32()?;
                Ok(raw::Operation::ReplaceLoan { loan, value })
            }
            0x44 => {
                let base = self.place_or_loan()?;
                let index = self.u32()?;
                let failure = self.u32()?;
                Ok(raw::Operation::IndexCopy { base, index, failure })
            }
            _ => Err(invalid()),
        }
    }
}
