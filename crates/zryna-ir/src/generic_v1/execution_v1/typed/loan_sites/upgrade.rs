//! Authenticated original terminator site is mandatory before lexical loan use or cleanup.
use super::{Failure, at, budget, raw, reject};
use zryna_source::UntrustedSpan;
pub(in crate::generic_v1::execution_v1::typed) fn check(
    f: &raw::Function,
    t: &raw::Terminator,
    site: UntrustedSpan,
    spent: &mut usize,
) -> Result<(), Failure> {
    let mut work = *spent;
    let raw::Terminator::WeakUpgradeBranch { weak, failure, .. } = t else {
        return Err(reject("upgrade site requires exact Weak branch opcode"));
    };
    budget::charge(&mut work, 1)?;
    let plan = f
        .cleanup_plans
        .get(*failure as usize)
        .ok_or_else(|| reject("upgrade failure cleanup is foreign"))?;
    if plan.span != site {
        return Err(reject("upgrade failure span differs from retained original use"));
    }
    if let raw::PlaceOrLoan::Loan(id) = weak {
        at(f, *id, site, &mut work)?;
    }
    budget::charge(&mut work, plan.actions.len())?;
    for action in &plan.actions {
        if let raw::CleanupAction::EndLoan(id) = action {
            at(f, *id, site, &mut work)?;
        }
    }
    *spent = work;
    Ok(())
}

#[cfg(test)]
mod tests;
