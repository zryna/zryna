//! Exact frozen `construct_operations` field order and explicit closed tags.
use super::{Failure, invalid, raw, reader::Reader};
use crate::data_ownership_v1 as m3;
impl Reader<'_> {
    pub(super) fn operation_construct(&mut self, tag: u8) -> Result<raw::Operation, Failure> {
        match tag {
            0x30 => {
                let ty = self.u32()?;
                let fields = self.vector(1024, 4, super::limits::Totals::aggregate, Self::u32)?;
                Ok(raw::Operation::StructConstruct { ty, fields })
            }
            0x31 => {
                let ty = self.u32()?;
                let ordinal = self.u32()?;
                let payload = self.option(Self::u32)?;
                self.totals.aggregate(usize::from(payload.is_some()))?;
                Ok(raw::Operation::ClosedEnumConstruct { ty, ordinal, payload })
            }
            0x32 => {
                let ty = self.u32()?;
                let elements = self.vector(
                    m3::MAX_AGGREGATE_OPERANDS,
                    4,
                    super::limits::Totals::aggregate,
                    Self::u32,
                )?;
                Ok(raw::Operation::FixedArrayConstruct { ty, elements })
            }
            0x33 => {
                let bytes = self.literal()?;
                let failure = self.u32()?;
                Ok(raw::Operation::StringFromUtf8 { bytes, failure })
            }
            0x34 => {
                let left = self.place_or_loan()?;
                let right = self.place_or_loan()?;
                let failure = self.u32()?;
                Ok(raw::Operation::StringConcat { left, right, failure })
            }
            0x35 => {
                let ty = self.u32()?;
                let elements = self.vector(
                    m3::MAX_AGGREGATE_OPERANDS,
                    4,
                    super::limits::Totals::aggregate,
                    Self::u32,
                )?;
                let failure = self.u32()?;
                Ok(raw::Operation::VecConstruct { ty, elements, failure })
            }
            0x36 => {
                let vector = self.place_or_loan()?;
                let value = self.u32()?;
                let failure = self.u32()?;
                Ok(raw::Operation::VecPush { vector, value, failure })
            }
            0x37 => {
                let ty = self.u32()?;
                let value = self.u32()?;
                let failure = self.u32()?;
                Ok(raw::Operation::SharedConstruct { ty, value, failure })
            }
            0x38 => {
                let shared = self.place_or_loan()?;
                let failure = self.u32()?;
                Ok(raw::Operation::WeakDowngrade { shared, failure })
            }
            _ => Err(invalid()),
        }
    }
}
