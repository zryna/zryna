//! Exact frozen operations field order and explicit closed tags.
use super::{Failure, invalid, raw, reader::Reader};
impl Reader<'_> {
    pub(super) fn scalar_binary_kind(&mut self) -> Result<raw::ScalarBinaryKind, Failure> {
        match self.byte()? {
            0x00 => Ok(raw::ScalarBinaryKind::Add),
            0x01 => Ok(raw::ScalarBinaryKind::Sub),
            0x02 => Ok(raw::ScalarBinaryKind::Mul),
            0x03 => Ok(raw::ScalarBinaryKind::Eq),
            0x04 => Ok(raw::ScalarBinaryKind::Ne),
            0x05 => Ok(raw::ScalarBinaryKind::LtS),
            0x06 => Ok(raw::ScalarBinaryKind::LeS),
            0x07 => Ok(raw::ScalarBinaryKind::GtS),
            0x08 => Ok(raw::ScalarBinaryKind::GeS),
            _ => Err(invalid()),
        }
    }
    pub(super) fn call_argument(&mut self) -> Result<raw::CallArgument, Failure> {
        match self.byte()? {
            0x00 => {
                let field0 = self.u32()?;
                Ok(raw::CallArgument::Value(field0))
            }
            0x01 => {
                let field0 = self.u32()?;
                Ok(raw::CallArgument::Loan(field0))
            }
            _ => Err(invalid()),
        }
    }
    pub(super) fn read_mode(&mut self) -> Result<raw::ReadMode, Failure> {
        match self.byte()? {
            0x00 => Ok(raw::ReadMode::Copy),
            0x01 => Ok(raw::ReadMode::Move),
            _ => Err(invalid()),
        }
    }

    pub(super) fn operation(&mut self) -> Result<raw::Operation, Failure> {
        let tag = self.byte()?;
        match tag {
            0x00..=0x04 => self.operation_scalar(tag),
            0x10..=0x14 => self.operation_place(tag),
            0x20..=0x21 => self.operation_call(tag),
            0x30..=0x38 => self.operation_construct(tag),
            0x40..=0x44 => self.operation_loan(tag),
            _ => Err(invalid()),
        }
    }
}
