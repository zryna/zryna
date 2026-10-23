//! Closed extension vocabulary per distinct envelope; core-v1 tags remain frozen.
use super::{Domain, Failure, Reader, Writer, budget, invalid, raw, reserve};

impl Writer {
    pub(super) fn operation_for(
        &mut self,
        operation: &raw::Operation,
        domain: Domain,
    ) -> Result<(), Failure> {
        match operation {
            raw::Operation::StringLiteral(bytes) => {
                self.bytes(&[1])?;
                self.u32(bytes.len())?;
                self.bytes(bytes)
            }
            raw::Operation::Move(id) => {
                self.bytes(&[2])?;
                self.bytes(&id.to_le_bytes())
            }
            raw::Operation::Borrow { value, exclusive } => {
                self.bytes(&[3])?;
                self.bytes(&value.to_le_bytes())?;
                self.bytes(&[u8::from(*exclusive)])
            }
            raw::Operation::EndLoan(id) => {
                self.bytes(&[4])?;
                self.bytes(&id.to_le_bytes())
            }
            raw::Operation::Drop(id) => {
                self.bytes(&[5])?;
                self.bytes(&id.to_le_bytes())
            }
            raw::Operation::CloneString(id) => {
                self.bytes(&[6])?;
                self.bytes(&id.to_le_bytes())
            }
            raw::Operation::CloneBorrowedString(id) => {
                if domain == Domain::V2 {
                    return Err(invalid());
                }
                self.bytes(&[7])?;
                self.bytes(&id.to_le_bytes())
            }
            raw::Operation::StructConstruct { ty, fields } => {
                if !matches!(domain, Domain::V4 | Domain::V5) {
                    return Err(invalid());
                }
                if fields.len() > zryna_layout::MAX_MEMBERS_PER_DECLARATION {
                    return Err(budget("owned wire Struct field count ceiling"));
                }
                self.bytes(&[8])?;
                self.bytes(&ty.to_le_bytes())?;
                self.ids(fields)
            }
            raw::Operation::CopyStructField { value, ordinal } => {
                if !matches!(domain, Domain::V4 | Domain::V5) {
                    return Err(invalid());
                }
                self.bytes(&[9])?;
                self.bytes(&value.to_le_bytes())?;
                self.bytes(&ordinal.to_le_bytes())
            }
            raw::Operation::CloneStruct { value } => {
                if domain != Domain::V5 {
                    return Err(invalid());
                }
                self.bytes(&[10])?;
                self.bytes(&value.to_le_bytes())
            }
        }
    }
}

impl Reader<'_> {
    pub(super) fn operation_for(&mut self, domain: Domain) -> Result<raw::Operation, Failure> {
        Ok(match self.take(1)?[0] {
            1 => {
                let n = self.u32()? as usize;
                if n > 65_536 {
                    return Err(budget("owned String literal ceiling"));
                }
                let bytes = self.take(n)?;
                std::str::from_utf8(bytes).map_err(|_| invalid())?;
                let mut v = reserve(n)?;
                v.extend_from_slice(bytes);
                raw::Operation::StringLiteral(v)
            }
            2 => raw::Operation::Move(self.u32()?),
            3 => raw::Operation::Borrow { value: self.u32()?, exclusive: self.boolean()? },
            4 => raw::Operation::EndLoan(self.u32()?),
            5 => raw::Operation::Drop(self.u32()?),
            6 => raw::Operation::CloneString(self.u32()?),
            7 if domain != Domain::V2 => raw::Operation::CloneBorrowedString(self.u32()?),
            8 if matches!(domain, Domain::V4 | Domain::V5) => raw::Operation::StructConstruct {
                ty: self.u32()?,
                fields: self.vector(zryna_layout::MAX_MEMBERS_PER_DECLARATION, 4, Self::u32)?,
            },
            9 if matches!(domain, Domain::V4 | Domain::V5) => {
                raw::Operation::CopyStructField { value: self.u32()?, ordinal: self.u32()? }
            }
            10 if domain == Domain::V5 => raw::Operation::CloneStruct { value: self.u32()? },
            _ => return Err(invalid()),
        })
    }
}
