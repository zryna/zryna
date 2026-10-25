//! Exact verified original-AST match facts, recorded during replay and indexed once by block.
use super::{Failure, charge, raw, reject};
use crate::generic_v1::execution_v1::{reserve, resource_failure};
use zryna_source::UntrustedSpan;
mod scopes;
pub(in crate::generic_v1::execution_v1) use scopes::Scope;
#[derive(Debug)]
pub(in crate::generic_v1::execution_v1::source) struct Arm {
    pub(in crate::generic_v1::execution_v1::source) ordinal: u32,
    pub(in crate::generic_v1::execution_v1::source) region: UntrustedSpan,
    pub(in crate::generic_v1::execution_v1::source) binding: Option<raw::LoanDefinition>,
    pub(in crate::generic_v1::execution_v1::source) blocks: std::ops::Range<usize>,
}
#[derive(Debug)]
pub(in crate::generic_v1::execution_v1::source) struct Use {
    pub(in crate::generic_v1::execution_v1::source) block: usize,
    pub(in crate::generic_v1::execution_v1::source) span: UntrustedSpan,
    pub(in crate::generic_v1::execution_v1::source) ty: u32,
    pub(in crate::generic_v1::execution_v1::source) parent: u32,
    pub(in crate::generic_v1::execution_v1::source) mode: raw::MatchMode,
    pub(in crate::generic_v1::execution_v1::source) arms: Vec<Arm>,
    pub(in crate::generic_v1::execution_v1::source) join: usize,
}
#[derive(Debug)]
pub(in crate::generic_v1::execution_v1::source) struct Matches(Vec<Vec<Option<Use>>>);
impl Matches {
    pub(in crate::generic_v1::execution_v1::source) fn new(
        functions: usize,
        work: &mut usize,
    ) -> Result<Self, Failure> {
        charge(
            work,
            functions
                .checked_mul(size_of::<Vec<Option<Use>>>() + 1)
                .ok_or_else(|| resource_failure("source match function census overflow"))?,
        )?;
        Ok(Self(reserve(functions)?))
    }
    pub(in crate::generic_v1::execution_v1::source) fn function(
        &mut self,
        f: &raw::Function,
        uses: Vec<Use>,
        work: &mut usize,
    ) -> Result<(), Failure> {
        if self.0.len() >= self.0.capacity() {
            return Err(Failure::InternalFailure);
        }
        if uses.is_empty() {
            charge(work, 1)?;
            self.0.push(Vec::new());
            return Ok(());
        }

        let cost = f
            .blocks
            .len()
            .checked_mul(size_of::<Option<Use>>() + 1)
            .and_then(|n| n.checked_add(uses.len().checked_mul(2)?))
            .ok_or_else(|| resource_failure("source match block census overflow"))?;
        charge(work, cost)?;
        let mut rows = reserve(f.blocks.len())?;
        rows.resize_with(f.blocks.len(), || None);
        for site in uses {
            let slot = rows.get_mut(site.block).ok_or(Failure::InternalFailure)?;
            if slot.replace(site).is_some() {
                return Err(reject("duplicate original match use site"));
            }
        }
        if self.0.len() >= self.0.capacity() {
            return Err(Failure::InternalFailure);
        }
        self.0.push(rows);
        Ok(())
    }
    pub(super) fn site(
        &self,
        function: usize,
        block: usize,
        f: &raw::Function,
        t: &raw::Terminator,
        spent: &mut usize,
    ) -> Result<UntrustedSpan, Failure> {
        let mut work = *spent;
        charge(&mut work, 3)?;
        let site = self
            .0
            .get(function)
            .and_then(|rows| rows.get(block))
            .and_then(Option::as_ref)
            .ok_or_else(|| reject("borrowed match has no retained original AST use"))?;
        let raw::Terminator::ClosedEnumMatch { ty, scrutinee, mode, arms } = t else {
            return Err(reject("original match fact needs exact match opcode"));
        };
        if *ty != site.ty
            || *scrutinee != raw::PlaceOrLoan::Loan(site.parent)
            || *mode != site.mode
            || arms.len() != site.arms.len()
        {
            return Err(reject("borrowed match differs from retained original source identity"));
        }
        charge(
            &mut work,
            arms.len()
                .checked_mul(4)
                .ok_or_else(|| resource_failure("source match arm comparison overflow"))?,
        )?;
        for (arm, original) in arms.iter().zip(&site.arms) {
            let binding = match &arm.binding {
                None => None,
                Some(raw::MatchBinding::Loan(d)) => Some(d),
                Some(raw::MatchBinding::Value(_)) => {
                    return Err(reject("borrowed source cannot issue owned match binding"));
                }
            };
            if arm.ordinal != original.ordinal || binding != original.binding.as_ref() {
                return Err(reject("selected binding differs from original arm"));
            }
            if let Some(binding) = binding {
                let row = f
                    .loans
                    .get(binding.id as usize)
                    .ok_or_else(|| reject("source selected loan row absent"))?;
                if row.region.span != original.region
                    || row.region.role != raw::RegionRole::SelectedMatchArm
                {
                    return Err(reject(
                        "selected region differs from exact original arm expression",
                    ));
                }
            }
        }
        *spent = work;
        Ok(site.span)
    }
}
