//! Exact frozen `scalar_operations` field order and explicit closed tags.
use super::{Failure, invalid, raw, writer::Writer};
impl Writer {
    pub(super) fn operation_scalar(&mut self, value: &raw::Operation) -> Result<(), Failure> {
        match value {
            raw::Operation::Unit => {
                self.byte(0x00)?;
                Ok(())
            }
            raw::Operation::BoolLiteral(field0) => {
                self.byte(0x01)?;
                self.boolean(*field0)?;
                Ok(())
            }
            raw::Operation::I32Literal(field0) => {
                self.byte(0x02)?;
                self.bytes(&field0.to_le_bytes())?;
                Ok(())
            }
            raw::Operation::ScalarBinary { kind, left, right } => {
                self.byte(0x03)?;
                self.scalar_binary_kind(*kind)?;
                self.u32(*left)?;
                self.u32(*right)?;
                Ok(())
            }
            raw::Operation::I32Neg { value } => {
                self.byte(0x04)?;
                self.u32(*value)?;
                Ok(())
            }
            _ => Err(invalid()),
        }
    }
}
