//! Exact frozen model field order and explicit closed tags.
use super::{Failure, raw, writer::Writer};
impl Writer {
    pub(super) fn ty(&mut self, value: raw::ValueType) -> Result<(), Failure> {
        match value {
            raw::ValueType::Stored(field0) => {
                self.byte(0x00)?;
                self.u32(field0)?;
                Ok(())
            }
            raw::ValueType::Unit => {
                self.byte(0x01)?;
                Ok(())
            }
        }
    }
    pub(super) fn value_definition(&mut self, value: &raw::ValueDefinition) -> Result<(), Failure> {
        self.u32(value.id)?;
        self.ty(value.ty)?;
        Ok(())
    }
    pub(super) fn loan_definition(&mut self, value: &raw::LoanDefinition) -> Result<(), Failure> {
        self.u32(value.id)?;
        self.u32(value.referent)?;
        self.access(value.access)?;
        Ok(())
    }
    pub(super) fn access(&mut self, value: raw::Access) -> Result<(), Failure> {
        match value {
            raw::Access::Shared => {
                self.byte(0x00)?;
                Ok(())
            }
            raw::Access::Exclusive => {
                self.byte(0x01)?;
                Ok(())
            }
        }
    }
    pub(super) fn parameter(&mut self, value: &raw::Parameter) -> Result<(), Failure> {
        match value {
            raw::Parameter::Value(field0) => {
                self.byte(0x00)?;
                self.value_definition(field0)?;
                Ok(())
            }
            raw::Parameter::Loan(field0) => {
                self.byte(0x01)?;
                self.loan_definition(field0)?;
                Ok(())
            }
        }
    }
    pub(super) fn place_kind(&mut self, value: &raw::PlaceKind) -> Result<(), Failure> {
        match value {
            raw::PlaceKind::Parameter(field0) => {
                self.byte(0x00)?;
                self.u32(*field0)?;
                Ok(())
            }
            raw::PlaceKind::Local(field0) => {
                self.byte(0x01)?;
                self.u32(*field0)?;
                Ok(())
            }
            raw::PlaceKind::Temporary(field0) => {
                self.byte(0x02)?;
                self.u32(*field0)?;
                Ok(())
            }
            raw::PlaceKind::StructField { base, ordinal } => {
                self.byte(0x03)?;
                self.u32(*base)?;
                self.u32(*ordinal)?;
                Ok(())
            }
            raw::PlaceKind::EnumPayload { base, variant } => {
                self.byte(0x04)?;
                self.u32(*base)?;
                self.u32(*variant)?;
                Ok(())
            }
            raw::PlaceKind::FixedArrayConstant { base, index } => {
                self.byte(0x05)?;
                self.u32(*base)?;
                self.u32(*index)?;
                Ok(())
            }
        }
    }
    pub(super) fn static_projection(
        &mut self,
        value: raw::StaticProjection,
    ) -> Result<(), Failure> {
        match value {
            raw::StaticProjection::StructField(field0) => {
                self.byte(0x00)?;
                self.u32(field0)?;
                Ok(())
            }
            raw::StaticProjection::EnumPayload(field0) => {
                self.byte(0x01)?;
                self.u32(field0)?;
                Ok(())
            }
            raw::StaticProjection::FixedArrayConstant(field0) => {
                self.byte(0x02)?;
                self.u32(field0)?;
                Ok(())
            }
        }
    }
    pub(super) fn place_or_loan(&mut self, value: raw::PlaceOrLoan) -> Result<(), Failure> {
        match value {
            raw::PlaceOrLoan::Place(field0) => {
                self.byte(0x00)?;
                self.u32(field0)?;
                Ok(())
            }
            raw::PlaceOrLoan::Loan(field0) => {
                self.byte(0x01)?;
                self.u32(field0)?;
                Ok(())
            }
        }
    }
    pub(super) fn loan_origin(&mut self, value: &raw::LoanOrigin) -> Result<(), Failure> {
        match value {
            raw::LoanOrigin::Formal(field0) => {
                self.byte(0x00)?;
                self.u32(*field0)?;
                Ok(())
            }
            raw::LoanOrigin::Place(field0) => {
                self.byte(0x01)?;
                self.u32(*field0)?;
                Ok(())
            }
            raw::LoanOrigin::InternalReborrow(field0) => {
                self.byte(0x02)?;
                self.u32(*field0)?;
                Ok(())
            }
            raw::LoanOrigin::StaticProjection { parent, projection } => {
                self.byte(0x03)?;
                self.u32(*parent)?;
                self.static_projection(*projection)?;
                Ok(())
            }
            raw::LoanOrigin::Indexed { base, index } => {
                self.byte(0x04)?;
                self.place_or_loan(*base)?;
                self.u32(*index)?;
                Ok(())
            }
        }
    }
    pub(super) fn region_role(&mut self, value: raw::RegionRole) -> Result<(), Failure> {
        match value {
            raw::RegionRole::Lexical => {
                self.byte(0x00)?;
                Ok(())
            }
            raw::RegionRole::SelectedMatchArm => {
                self.byte(0x01)?;
                Ok(())
            }
            raw::RegionRole::TransientIndexedAccess => {
                self.byte(0x02)?;
                Ok(())
            }
        }
    }
    pub(super) fn place(&mut self, value: &raw::Place) -> Result<(), Failure> {
        self.u32(value.id)?;
        self.u32(value.ty)?;
        self.span(&value.span)?;
        self.place_kind(&value.kind)?;
        Ok(())
    }
    pub(super) fn region(&mut self, value: &raw::Region) -> Result<(), Failure> {
        self.span(&value.span)?;
        self.region_role(value.role)?;
        Ok(())
    }
    pub(super) fn loan(&mut self, value: &raw::Loan) -> Result<(), Failure> {
        self.u32(value.id)?;
        self.u32(value.referent)?;
        self.access(value.access)?;
        self.region(&value.region)?;
        self.loan_origin(&value.origin)?;
        Ok(())
    }
}
