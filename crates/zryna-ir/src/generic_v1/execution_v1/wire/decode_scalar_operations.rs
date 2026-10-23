//! Exact frozen `scalar_operations` field order and explicit closed tags.
use super::{Failure, invalid, raw, reader::Reader};
impl Reader<'_> {
    pub(super) fn operation_scalar(&mut self, tag: u8) -> Result<raw::Operation, Failure> {
        match tag {
            0x00 => Ok(raw::Operation::Unit),
            0x01 => {
                let field0 = self.boolean()?;
                Ok(raw::Operation::BoolLiteral(field0))
            }
            0x02 => {
                let field0 = self.i32()?;
                Ok(raw::Operation::I32Literal(field0))
            }
            0x03 => {
                let kind = self.scalar_binary_kind()?;
                let left = self.u32()?;
                let right = self.u32()?;
                Ok(raw::Operation::ScalarBinary { kind, left, right })
            }
            0x04 => {
                let value = self.u32()?;
                Ok(raw::Operation::I32Neg { value })
            }
            _ => Err(invalid()),
        }
    }
}
