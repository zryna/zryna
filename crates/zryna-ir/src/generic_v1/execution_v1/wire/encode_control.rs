//! Exact frozen control field order and explicit closed tags.
use super::{Failure, raw, writer::Writer};
impl Writer {
    pub(super) fn match_mode(&mut self, value: raw::MatchMode) -> Result<(), Failure> {
        match value {
            raw::MatchMode::Value => {
                self.byte(0x00)?;
                Ok(())
            }
            raw::MatchMode::SharedLoan => {
                self.byte(0x01)?;
                Ok(())
            }
            raw::MatchMode::ExclusiveLoan => {
                self.byte(0x02)?;
                Ok(())
            }
        }
    }
    pub(super) fn match_binding(&mut self, value: &raw::MatchBinding) -> Result<(), Failure> {
        match value {
            raw::MatchBinding::Value(field0) => {
                self.byte(0x00)?;
                self.value_definition(field0)?;
                Ok(())
            }
            raw::MatchBinding::Loan(field0) => {
                self.byte(0x01)?;
                self.loan_definition(field0)?;
                Ok(())
            }
        }
    }
    pub(super) fn trap_identity(&mut self, value: raw::TrapIdentity) -> Result<(), Failure> {
        match value {
            raw::TrapIdentity::Bounds => {
                self.byte(0x00)?;
                Ok(())
            }
            raw::TrapIdentity::Allocation => {
                self.byte(0x01)?;
                Ok(())
            }
            raw::TrapIdentity::Capacity => {
                self.byte(0x02)?;
                Ok(())
            }
            raw::TrapIdentity::Refcount => {
                self.byte(0x03)?;
                Ok(())
            }
            raw::TrapIdentity::Utf8 => {
                self.byte(0x04)?;
                Ok(())
            }
        }
    }
    pub(super) fn cleanup_action(&mut self, value: &raw::CleanupAction) -> Result<(), Failure> {
        match value {
            raw::CleanupAction::EndLoan(field0) => {
                self.byte(0x00)?;
                self.u32(*field0)?;
                Ok(())
            }
            raw::CleanupAction::DropPlace(field0) => {
                self.byte(0x01)?;
                self.u32(*field0)?;
                Ok(())
            }
            raw::CleanupAction::DropInitializedPrefix { destination, preparation_site } => {
                self.byte(0x02)?;
                self.u32(*destination)?;
                self.preparation_site(preparation_site)?;
                Ok(())
            }
        }
    }
    pub(super) fn edge(&mut self, value: &raw::Edge) -> Result<(), Failure> {
        self.u32(value.target)?;
        self.ids(&value.arguments)?;
        Ok(())
    }
    pub(super) fn enum_arm(&mut self, value: &raw::EnumArm) -> Result<(), Failure> {
        self.u32(value.ordinal)?;
        self.option(value.binding.as_ref(), Self::match_binding)?;
        self.edge(&value.edge)?;
        Ok(())
    }
    pub(super) fn preparation_site(&mut self, value: &raw::PreparationSite) -> Result<(), Failure> {
        self.u32(value.function)?;
        self.u32(value.block)?;
        self.u32(value.instruction)?;
        Ok(())
    }
    pub(super) fn cleanup_plan(&mut self, value: &raw::CleanupPlan) -> Result<(), Failure> {
        self.u32(value.id)?;
        self.span(&value.span)?;
        self.vector(&value.actions, Self::cleanup_action)?;
        Ok(())
    }
    pub(super) fn terminator(&mut self, value: &raw::Terminator) -> Result<(), Failure> {
        match value {
            raw::Terminator::Return { value, cleanup } => {
                self.byte(0x00)?;
                self.optional_id(*value)?;
                self.u32(*cleanup)?;
                Ok(())
            }
            raw::Terminator::Jump(field0) => {
                self.byte(0x01)?;
                self.edge(field0)?;
                Ok(())
            }
            raw::Terminator::Branch { condition, yes, no } => {
                self.byte(0x02)?;
                self.u32(*condition)?;
                self.edge(yes)?;
                self.edge(no)?;
                Ok(())
            }
            raw::Terminator::ClosedEnumMatch { ty, scrutinee, mode, arms } => {
                self.byte(0x03)?;
                self.u32(*ty)?;
                self.place_or_loan(*scrutinee)?;
                self.match_mode(*mode)?;
                self.vector(arms, Self::enum_arm)?;
                Ok(())
            }
            raw::Terminator::WeakUpgradeBranch {
                weak,
                success_binding,
                success,
                expired,
                failure,
            } => {
                self.byte(0x04)?;
                self.place_or_loan(*weak)?;
                self.value_definition(success_binding)?;
                self.edge(success)?;
                self.edge(expired)?;
                self.u32(*failure)?;
                Ok(())
            }
            raw::Terminator::Trap { identity, cleanup } => {
                self.byte(0x05)?;
                self.trap_identity(*identity)?;
                self.u32(*cleanup)?;
                Ok(())
            }
        }
    }
}
