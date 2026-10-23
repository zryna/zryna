//! Exact frozen operations field order and explicit closed tags.
use super::{Failure, raw, writer::Writer};
impl Writer {
    pub(super) fn scalar_binary_kind(
        &mut self,
        value: raw::ScalarBinaryKind,
    ) -> Result<(), Failure> {
        match value {
            raw::ScalarBinaryKind::Add => {
                self.byte(0x00)?;
                Ok(())
            }
            raw::ScalarBinaryKind::Sub => {
                self.byte(0x01)?;
                Ok(())
            }
            raw::ScalarBinaryKind::Mul => {
                self.byte(0x02)?;
                Ok(())
            }
            raw::ScalarBinaryKind::Eq => {
                self.byte(0x03)?;
                Ok(())
            }
            raw::ScalarBinaryKind::Ne => {
                self.byte(0x04)?;
                Ok(())
            }
            raw::ScalarBinaryKind::LtS => {
                self.byte(0x05)?;
                Ok(())
            }
            raw::ScalarBinaryKind::LeS => {
                self.byte(0x06)?;
                Ok(())
            }
            raw::ScalarBinaryKind::GtS => {
                self.byte(0x07)?;
                Ok(())
            }
            raw::ScalarBinaryKind::GeS => {
                self.byte(0x08)?;
                Ok(())
            }
        }
    }
    pub(super) fn call_argument(&mut self, value: raw::CallArgument) -> Result<(), Failure> {
        match value {
            raw::CallArgument::Value(field0) => {
                self.byte(0x00)?;
                self.u32(field0)?;
                Ok(())
            }
            raw::CallArgument::Loan(field0) => {
                self.byte(0x01)?;
                self.u32(field0)?;
                Ok(())
            }
        }
    }
    pub(super) fn read_mode(&mut self, value: raw::ReadMode) -> Result<(), Failure> {
        match value {
            raw::ReadMode::Copy => {
                self.byte(0x00)?;
                Ok(())
            }
            raw::ReadMode::Move => {
                self.byte(0x01)?;
                Ok(())
            }
        }
    }

    pub(super) fn operation(&mut self, value: &raw::Operation) -> Result<(), Failure> {
        match value {
            raw::Operation::Unit
            | raw::Operation::BoolLiteral(..)
            | raw::Operation::I32Literal(..)
            | raw::Operation::ScalarBinary { .. }
            | raw::Operation::I32Neg { .. } => self.operation_scalar(value),
            raw::Operation::ReadPlace { .. }
            | raw::Operation::InitializePlace { .. }
            | raw::Operation::ReplacePlace { .. }
            | raw::Operation::DropPlace { .. }
            | raw::Operation::Clone { .. } => self.operation_place(value),
            raw::Operation::ClosedGenericCall { .. } | raw::Operation::SourceCall { .. } => {
                self.operation_call(value)
            }
            raw::Operation::StructConstruct { .. }
            | raw::Operation::ClosedEnumConstruct { .. }
            | raw::Operation::FixedArrayConstruct { .. }
            | raw::Operation::StringFromUtf8 { .. }
            | raw::Operation::StringConcat { .. }
            | raw::Operation::VecConstruct { .. }
            | raw::Operation::VecPush { .. }
            | raw::Operation::SharedConstruct { .. }
            | raw::Operation::WeakDowngrade { .. } => self.operation_construct(value),
            raw::Operation::BeginLoan { .. }
            | raw::Operation::EndLoan { .. }
            | raw::Operation::ReadLoan { .. }
            | raw::Operation::ReplaceLoan { .. }
            | raw::Operation::IndexCopy { .. } => self.operation_loan(value),
        }
    }
}
