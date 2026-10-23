//! Exact frozen `call_operations` field order and explicit closed tags.
use super::{Failure, invalid, raw, writer::Writer};
impl Writer {
    pub(super) fn operation_call(&mut self, value: &raw::Operation) -> Result<(), Failure> {
        match value {
            raw::Operation::ClosedGenericCall { instance, arguments, failure } => {
                self.byte(0x20)?;
                self.u32(*instance)?;
                self.vector(arguments, |writer, value| writer.call_argument(*value))?;
                self.optional_id(*failure)?;
                Ok(())
            }
            raw::Operation::SourceCall { module, function, arguments, failure } => {
                self.byte(0x21)?;
                self.u32(*module)?;
                self.u32(*function)?;
                self.vector(arguments, |writer, value| writer.call_argument(*value))?;
                self.optional_id(*failure)?;
                Ok(())
            }
            _ => Err(invalid()),
        }
    }
}
