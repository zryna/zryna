//! Exact frozen `construct_operations` field order and explicit closed tags.
use super::{Failure, invalid, raw, writer::Writer};
impl Writer {
    pub(super) fn operation_construct(&mut self, value: &raw::Operation) -> Result<(), Failure> {
        match value {
            raw::Operation::StructConstruct { ty, fields } => {
                self.byte(0x30)?;
                self.u32(*ty)?;
                self.vector(fields, |writer, value| writer.u32(*value))?;
                Ok(())
            }
            raw::Operation::ClosedEnumConstruct { ty, ordinal, payload } => {
                self.byte(0x31)?;
                self.u32(*ty)?;
                self.u32(*ordinal)?;
                self.optional_id(*payload)?;
                Ok(())
            }
            raw::Operation::FixedArrayConstruct { ty, elements } => {
                self.byte(0x32)?;
                self.u32(*ty)?;
                self.vector(elements, |writer, value| writer.u32(*value))?;
                Ok(())
            }
            raw::Operation::StringFromUtf8 { bytes, failure } => {
                self.byte(0x33)?;
                self.blob(bytes)?;
                self.u32(*failure)?;
                Ok(())
            }
            raw::Operation::StringConcat { left, right, failure } => {
                self.byte(0x34)?;
                self.place_or_loan(*left)?;
                self.place_or_loan(*right)?;
                self.u32(*failure)?;
                Ok(())
            }
            raw::Operation::VecConstruct { ty, elements, failure } => {
                self.byte(0x35)?;
                self.u32(*ty)?;
                self.vector(elements, |writer, value| writer.u32(*value))?;
                self.u32(*failure)?;
                Ok(())
            }
            raw::Operation::VecPush { vector, value, failure } => {
                self.byte(0x36)?;
                self.place_or_loan(*vector)?;
                self.u32(*value)?;
                self.u32(*failure)?;
                Ok(())
            }
            raw::Operation::SharedConstruct { ty, value, failure } => {
                self.byte(0x37)?;
                self.u32(*ty)?;
                self.u32(*value)?;
                self.u32(*failure)?;
                Ok(())
            }
            raw::Operation::WeakDowngrade { shared, failure } => {
                self.byte(0x38)?;
                self.place_or_loan(*shared)?;
                self.u32(*failure)?;
                Ok(())
            }
            _ => Err(invalid()),
        }
    }
}
