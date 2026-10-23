//! Exact frozen control field order and explicit closed tags.
use super::{Failure, invalid, raw, reader::Reader};
use crate::data_ownership_v1 as m3;
impl Reader<'_> {
    pub(super) fn match_mode(&mut self) -> Result<raw::MatchMode, Failure> {
        match self.byte()? {
            0x00 => Ok(raw::MatchMode::Value),
            0x01 => Ok(raw::MatchMode::SharedLoan),
            0x02 => Ok(raw::MatchMode::ExclusiveLoan),
            _ => Err(invalid()),
        }
    }
    pub(super) fn match_binding(&mut self) -> Result<raw::MatchBinding, Failure> {
        match self.byte()? {
            0x00 => {
                let field0 = self.value_definition()?;
                Ok(raw::MatchBinding::Value(field0))
            }
            0x01 => {
                let field0 = self.loan_definition()?;
                Ok(raw::MatchBinding::Loan(field0))
            }
            _ => Err(invalid()),
        }
    }
    pub(super) fn trap_identity(&mut self) -> Result<raw::TrapIdentity, Failure> {
        match self.byte()? {
            0x00 => Ok(raw::TrapIdentity::Bounds),
            0x01 => Ok(raw::TrapIdentity::Allocation),
            0x02 => Ok(raw::TrapIdentity::Capacity),
            0x03 => Ok(raw::TrapIdentity::Refcount),
            0x04 => Ok(raw::TrapIdentity::Utf8),
            _ => Err(invalid()),
        }
    }
    pub(super) fn cleanup_action(&mut self) -> Result<raw::CleanupAction, Failure> {
        match self.byte()? {
            0x00 => {
                let field0 = self.u32()?;
                Ok(raw::CleanupAction::EndLoan(field0))
            }
            0x01 => {
                let field0 = self.u32()?;
                Ok(raw::CleanupAction::DropPlace(field0))
            }
            0x02 => {
                let destination = self.u32()?;
                let preparation_site = self.preparation_site()?;
                Ok(raw::CleanupAction::DropInitializedPrefix { destination, preparation_site })
            }
            _ => Err(invalid()),
        }
    }
    pub(super) fn edge(&mut self) -> Result<raw::Edge, Failure> {
        self.totals.edge()?;
        self.edge_prepaid()
    }
    // Caller has already charged this exact edge before any enclosing arena reserve.
    fn edge_prepaid(&mut self) -> Result<raw::Edge, Failure> {
        Ok(raw::Edge { target: self.u32()?, arguments: self.ids()? })
    }
    #[cfg(test)]
    pub(super) fn enum_arm(&mut self) -> Result<raw::EnumArm, Failure> {
        self.totals.edge()?;
        self.enum_arm_prepaid()
    }
    fn enum_arm_prepaid(&mut self) -> Result<raw::EnumArm, Failure> {
        Ok(raw::EnumArm {
            ordinal: self.u32()?,
            binding: self.option(Self::match_binding)?,
            edge: self.edge_prepaid()?,
        })
    }
    pub(super) fn preparation_site(&mut self) -> Result<raw::PreparationSite, Failure> {
        Ok(raw::PreparationSite {
            function: self.u32()?,
            block: self.u32()?,
            instruction: self.u32()?,
        })
    }
    pub(super) fn cleanup_plan(&mut self) -> Result<raw::CleanupPlan, Failure> {
        Ok(raw::CleanupPlan {
            id: self.u32()?,
            span: self.span()?,
            actions: self.vector(
                m3::MAX_DROP_ACTIONS_PER_FUNCTION,
                5,
                super::limits::Totals::drops,
                Self::cleanup_action,
            )?,
        })
    }
    pub(super) fn terminator(&mut self) -> Result<raw::Terminator, Failure> {
        match self.byte()? {
            0x00 => {
                let value = self.option(Self::u32)?;
                let cleanup = self.u32()?;
                Ok(raw::Terminator::Return { value, cleanup })
            }
            0x01 => {
                let field0 = self.edge()?;
                Ok(raw::Terminator::Jump(field0))
            }
            0x02 => {
                let condition = self.u32()?;
                let yes = self.edge()?;
                let no = self.edge()?;
                Ok(raw::Terminator::Branch { condition, yes, no })
            }
            0x03 => {
                let ty = self.u32()?;
                let scrutinee = self.place_or_loan()?;
                let mode = self.match_mode()?;
                let arms =
                    self.vector(1024, 13, super::limits::Totals::edges, Self::enum_arm_prepaid)?;
                Ok(raw::Terminator::ClosedEnumMatch { ty, scrutinee, mode, arms })
            }
            0x04 => {
                let weak = self.place_or_loan()?;
                let success_binding = self.value_definition()?;
                let success = self.edge()?;
                let expired = self.edge()?;
                let failure = self.u32()?;
                Ok(raw::Terminator::WeakUpgradeBranch {
                    weak,
                    success_binding,
                    success,
                    expired,
                    failure,
                })
            }
            0x05 => {
                let identity = self.trap_identity()?;
                let cleanup = self.u32()?;
                Ok(raw::Terminator::Trap { identity, cleanup })
            }
            _ => Err(invalid()),
        }
    }
}
