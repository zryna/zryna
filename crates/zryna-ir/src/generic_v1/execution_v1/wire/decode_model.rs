//! Exact frozen model field order and explicit closed tags.
use super::{Failure, invalid, raw, reader::Reader};
impl Reader<'_> {
    pub(super) fn ty(&mut self) -> Result<raw::ValueType, Failure> {
        match self.byte()? {
            0x00 => {
                let field0 = self.u32()?;
                Ok(raw::ValueType::Stored(field0))
            }
            0x01 => Ok(raw::ValueType::Unit),
            _ => Err(invalid()),
        }
    }
    pub(super) fn value_definition(&mut self) -> Result<raw::ValueDefinition, Failure> {
        Ok(raw::ValueDefinition { id: self.u32()?, ty: self.ty()? })
    }
    pub(super) fn loan_definition(&mut self) -> Result<raw::LoanDefinition, Failure> {
        Ok(raw::LoanDefinition { id: self.u32()?, referent: self.u32()?, access: self.access()? })
    }
    pub(super) fn access(&mut self) -> Result<raw::Access, Failure> {
        match self.byte()? {
            0x00 => Ok(raw::Access::Shared),
            0x01 => Ok(raw::Access::Exclusive),
            _ => Err(invalid()),
        }
    }
    pub(super) fn parameter(&mut self) -> Result<raw::Parameter, Failure> {
        match self.byte()? {
            0x00 => {
                self.totals.values(1)?;
                let field0 = self.value_definition()?;
                Ok(raw::Parameter::Value(field0))
            }
            0x01 => {
                let field0 = self.loan_definition()?;
                Ok(raw::Parameter::Loan(field0))
            }
            _ => Err(invalid()),
        }
    }
    pub(super) fn place_kind(&mut self) -> Result<raw::PlaceKind, Failure> {
        match self.byte()? {
            0x00 => {
                let field0 = self.u32()?;
                Ok(raw::PlaceKind::Parameter(field0))
            }
            0x01 => {
                let field0 = self.u32()?;
                Ok(raw::PlaceKind::Local(field0))
            }
            0x02 => {
                let field0 = self.u32()?;
                Ok(raw::PlaceKind::Temporary(field0))
            }
            0x03 => {
                let base = self.u32()?;
                let ordinal = self.u32()?;
                Ok(raw::PlaceKind::StructField { base, ordinal })
            }
            0x04 => {
                let base = self.u32()?;
                let variant = self.u32()?;
                Ok(raw::PlaceKind::EnumPayload { base, variant })
            }
            0x05 => {
                let base = self.u32()?;
                let index = self.u32()?;
                Ok(raw::PlaceKind::FixedArrayConstant { base, index })
            }
            _ => Err(invalid()),
        }
    }
    pub(super) fn static_projection(&mut self) -> Result<raw::StaticProjection, Failure> {
        match self.byte()? {
            0x00 => {
                let field0 = self.u32()?;
                Ok(raw::StaticProjection::StructField(field0))
            }
            0x01 => {
                let field0 = self.u32()?;
                Ok(raw::StaticProjection::EnumPayload(field0))
            }
            0x02 => {
                let field0 = self.u32()?;
                Ok(raw::StaticProjection::FixedArrayConstant(field0))
            }
            _ => Err(invalid()),
        }
    }
    pub(super) fn place_or_loan(&mut self) -> Result<raw::PlaceOrLoan, Failure> {
        match self.byte()? {
            0x00 => {
                let field0 = self.u32()?;
                Ok(raw::PlaceOrLoan::Place(field0))
            }
            0x01 => {
                let field0 = self.u32()?;
                Ok(raw::PlaceOrLoan::Loan(field0))
            }
            _ => Err(invalid()),
        }
    }
    pub(super) fn loan_origin(&mut self) -> Result<raw::LoanOrigin, Failure> {
        match self.byte()? {
            0x00 => {
                let field0 = self.u32()?;
                Ok(raw::LoanOrigin::Formal(field0))
            }
            0x01 => {
                let field0 = self.u32()?;
                Ok(raw::LoanOrigin::Place(field0))
            }
            0x02 => {
                let field0 = self.u32()?;
                Ok(raw::LoanOrigin::InternalReborrow(field0))
            }
            0x03 => {
                let parent = self.u32()?;
                let projection = self.static_projection()?;
                Ok(raw::LoanOrigin::StaticProjection { parent, projection })
            }
            0x04 => {
                let base = self.place_or_loan()?;
                let index = self.u32()?;
                Ok(raw::LoanOrigin::Indexed { base, index })
            }
            _ => Err(invalid()),
        }
    }
    pub(super) fn region_role(&mut self) -> Result<raw::RegionRole, Failure> {
        match self.byte()? {
            0x00 => Ok(raw::RegionRole::Lexical),
            0x01 => Ok(raw::RegionRole::SelectedMatchArm),
            0x02 => Ok(raw::RegionRole::TransientIndexedAccess),
            _ => Err(invalid()),
        }
    }
    pub(super) fn place(&mut self) -> Result<raw::Place, Failure> {
        Ok(raw::Place {
            id: self.u32()?,
            ty: self.u32()?,
            span: self.span()?,
            kind: self.place_kind()?,
        })
    }
    pub(super) fn region(&mut self) -> Result<raw::Region, Failure> {
        Ok(raw::Region { span: self.span()?, role: self.region_role()? })
    }
    pub(super) fn loan(&mut self) -> Result<raw::Loan, Failure> {
        Ok(raw::Loan {
            id: self.u32()?,
            referent: self.u32()?,
            access: self.access()?,
            region: self.region()?,
            origin: self.loan_origin()?,
        })
    }
}
