//! Exact frozen `call_operations` field order and explicit closed tags.
use super::{Failure, invalid, raw, reader::Reader};
use crate::data_ownership_v1 as m3;
impl Reader<'_> {
    pub(super) fn operation_call(&mut self, tag: u8) -> Result<raw::Operation, Failure> {
        match tag {
            0x20 => {
                self.totals.call()?;
                let instance = self.u32()?;
                let arguments =
                    self.plain(m3::MAX_PARAMETERS_PER_FUNCTION, 5, Self::call_argument)?;
                let failure = self.option(Self::u32)?;
                Ok(raw::Operation::ClosedGenericCall { instance, arguments, failure })
            }
            0x21 => {
                self.totals.call()?;
                let module = self.u32()?;
                let function = self.u32()?;
                let arguments =
                    self.plain(m3::MAX_PARAMETERS_PER_FUNCTION, 5, Self::call_argument)?;
                let failure = self.option(Self::u32)?;
                Ok(raw::Operation::SourceCall { module, function, arguments, failure })
            }
            _ => Err(invalid()),
        }
    }
}
