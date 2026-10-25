//! Source-produced terminator use sites; raw claim cleanup spans never authenticate themselves.
use super::{Failure, charge, raw, reject};
use crate::generic_v1::execution_v1::{reserve, resource_failure};
use zryna_source::UntrustedSpan;
#[derive(Debug)]
struct Row {
    function: usize,
    block: usize,
    weak: raw::PlaceOrLoan,
    failure: u32,
    span: UntrustedSpan,
}
#[derive(Debug)]
pub(super) struct Upgrades(Vec<Row>);
impl Upgrades {
    pub(super) fn new(p: &raw::Program, spent: &mut usize) -> Result<Self, Failure> {
        let mut work = *spent;
        charge(
            &mut work,
            p.functions
                .len()
                .checked_mul(3)
                .ok_or_else(|| resource_failure("source upgrade function census overflow"))?,
        )?;
        let blocks = p
            .functions
            .iter()
            .try_fold(0usize, |n, f| n.checked_add(f.blocks.len()))
            .ok_or_else(|| resource_failure("source upgrade block census overflow"))?;
        charge(
            &mut work,
            blocks
                .checked_mul(2)
                .ok_or_else(|| resource_failure("source upgrade block work overflow"))?,
        )?;
        let count = p
            .functions
            .iter()
            .flat_map(|f| &f.blocks)
            .filter(|b| matches!(b.terminator, raw::Terminator::WeakUpgradeBranch { .. }))
            .count();
        charge(
            &mut work,
            count
                .checked_mul(2)
                .and_then(|n| n.checked_add(1))
                .ok_or_else(|| resource_failure("source upgrade retained census overflow"))?,
        )?;
        let mut rows = reserve(count)?;
        for (function, f) in p.functions.iter().enumerate() {
            for (block, b) in f.blocks.iter().enumerate() {
                if let raw::Terminator::WeakUpgradeBranch { weak, failure, .. } = b.terminator {
                    let plan = f
                        .cleanup_plans
                        .get(failure as usize)
                        .ok_or_else(|| reject("source upgrade failure site absent"))?;
                    rows.push(Row { function, block, weak, failure, span: plan.span });
                }
            }
        }
        *spent = work;
        Ok(Self(rows))
    }
    pub(super) fn site(
        &self,
        function: usize,
        block: usize,
        weak: raw::PlaceOrLoan,
        failure: u32,
        spent: &mut usize,
    ) -> Result<UntrustedSpan, Failure> {
        let mut work = *spent;
        let (mut lo, mut hi) = (0, self.0.len());
        while lo < hi {
            charge(&mut work, 1)?;
            let mid = lo + (hi - lo) / 2;
            let row = &self.0[mid];
            match (row.function, row.block).cmp(&(function, block)) {
                std::cmp::Ordering::Less => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
                std::cmp::Ordering::Equal => {
                    charge(&mut work, 1)?;
                    if row.weak != weak || row.failure != failure {
                        return Err(reject("upgrade differs from retained original source use"));
                    }
                    *spent = work;
                    return Ok(row.span);
                }
            }
        }
        Err(reject("upgrade has no retained original source use"))
    }
}

#[cfg(test)]
mod tests;
