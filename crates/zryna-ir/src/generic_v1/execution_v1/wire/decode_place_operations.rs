//! Exact frozen `place_operations` field order and explicit closed tags.
use super::{Failure, invalid, raw, reader::Reader};
impl Reader<'_> {
    pub(super) fn operation_place(&mut self, tag: u8) -> Result<raw::Operation, Failure> {
        match tag {
            0x10 => {
                let place = self.u32()?;
                let mode = self.read_mode()?;
                Ok(raw::Operation::ReadPlace { place, mode })
            }
            0x11 => {
                let place = self.u32()?;
                let value = self.u32()?;
                Ok(raw::Operation::InitializePlace { place, value })
            }
            0x12 => {
                let place = self.u32()?;
                let value = self.u32()?;
                Ok(raw::Operation::ReplacePlace { place, value })
            }
            0x13 => {
                self.totals.drops(1)?;
                let place = self.u32()?;
                Ok(raw::Operation::DropPlace { place })
            }
            0x14 => {
                let source = self.place_or_loan()?;
                let outer_failure = self.option(Self::u32)?;
                let prefix_failure = self.option(Self::u32)?;
                Ok(raw::Operation::Clone { source, outer_failure, prefix_failure })
            }
            _ => Err(invalid()),
        }
    }
}
