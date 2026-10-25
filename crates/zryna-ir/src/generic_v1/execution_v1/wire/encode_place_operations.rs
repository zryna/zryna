//! Exact frozen `place_operations` field order and explicit closed tags.
use super::{Failure, invalid, raw, writer::Writer};
impl Writer {
    pub(super) fn operation_place(&mut self, value: &raw::Operation) -> Result<(), Failure> {
        match value {
            raw::Operation::ReadPlace { place, mode } => {
                self.byte(0x10)?;
                self.u32(*place)?;
                self.read_mode(*mode)?;
                Ok(())
            }
            raw::Operation::InitializePlace { place, value } => {
                self.byte(0x11)?;
                self.u32(*place)?;
                self.u32(*value)?;
                Ok(())
            }
            raw::Operation::ReplacePlace { place, value } => {
                self.byte(0x12)?;
                self.u32(*place)?;
                self.u32(*value)?;
                Ok(())
            }
            raw::Operation::DropPlace { place } => {
                self.byte(0x13)?;
                self.u32(*place)?;
                Ok(())
            }
            raw::Operation::Clone { source, outer_failure, prefix_failure } => {
                self.byte(0x14)?;
                self.place_or_loan(*source)?;
                self.optional_id(*outer_failure)?;
                self.optional_id(*prefix_failure)?;
                Ok(())
            }
            _ => Err(invalid()),
        }
    }
}
