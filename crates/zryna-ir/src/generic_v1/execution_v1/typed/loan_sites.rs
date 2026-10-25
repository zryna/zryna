//! Instruction spans authenticate lexical loan use; unfinished terminator roles fail closed.
use super::{Failure, budget, raw, reject};
use crate::generic_v1::execution_v1::{places, resource_failure};
use zryna_source::UntrustedSpan;
mod failures;
mod matches;
pub(super) use matches::check as match_at;
#[cfg(test)]
mod tests;
mod upgrade;
pub(super) use upgrade::check as upgrade_at;

pub(super) fn instruction(
    f: &raw::Function,
    i: &raw::Instruction,
    spent: &mut usize,
) -> Result<(), Failure> {
    let mut work = *spent;
    inspect_instruction(f, i, &mut work)?;
    failures::check(f, i, &mut work)?;
    *spent = work;
    Ok(())
}
fn inspect_instruction(
    f: &raw::Function,
    i: &raw::Instruction,
    work: &mut usize,
) -> Result<(), Failure> {
    match &i.operation {
        raw::Operation::Clone { source: raw::PlaceOrLoan::Loan(id), .. }
        | raw::Operation::IndexCopy { base: raw::PlaceOrLoan::Loan(id), .. }
        | raw::Operation::WeakDowngrade { shared: raw::PlaceOrLoan::Loan(id), .. }
        | raw::Operation::VecPush { vector: raw::PlaceOrLoan::Loan(id), .. }
        | raw::Operation::EndLoan { loan: id }
        | raw::Operation::ReadLoan { loan: id }
        | raw::Operation::ReplaceLoan { loan: id, .. } => {
            at(f, *id, i.site_span, work)?;
        }
        raw::Operation::BeginLoan { loan, failure } => {
            let row = at(f, *loan, i.site_span, work)?;
            if failure.is_some() != matches!(row.origin, raw::LoanOrigin::Indexed { .. })
                || matches!(row.origin, raw::LoanOrigin::Formal(_))
                || row.region.role == raw::RegionRole::SelectedMatchArm
            {
                return Err(reject(
                    "loan instruction role differs from exact origin/failure shape",
                ));
            }
        }
        raw::Operation::StringConcat { left, right, .. } => {
            for source in [left, right] {
                if let raw::PlaceOrLoan::Loan(id) = source {
                    at(f, *id, i.site_span, work)?;
                }
            }
        }
        raw::Operation::ClosedGenericCall { arguments, .. }
        | raw::Operation::SourceCall { arguments, .. } => {
            budget::charge(work, arguments.len())?;
            for argument in arguments {
                if let raw::CallArgument::Loan(id) = argument {
                    at(f, *id, i.site_span, work)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}
fn at<'a>(
    f: &'a raw::Function,
    id: u32,
    site: UntrustedSpan,
    work: &mut usize,
) -> Result<&'a raw::Loan, Failure> {
    budget::charge(work, 1)?;
    let loan = f.loans.get(id as usize).ok_or_else(|| reject("instruction uses a foreign loan"))?;
    if loan.id != id || !places::within(loan.region.span, site) {
        return Err(reject("loan use is outside its exact lexical source region"));
    }
    role(loan)?;
    Ok(loan)
}

pub(super) fn terminator(
    f: &raw::Function,
    t: &raw::Terminator,
    spent: &mut usize,
) -> Result<(), Failure> {
    let mut work = *spent;
    match t {
        raw::Terminator::ClosedEnumMatch { scrutinee, mode, arms, .. } => {
            if matches!(scrutinee, raw::PlaceOrLoan::Loan(_)) || *mode != raw::MatchMode::Value {
                return Err(reject(
                    "terminator loan use requires a retained source-bound use fact",
                ));
            }
            budget::charge(&mut work, arms.len())?;
            if arms.iter().any(|arm| matches!(arm.binding, Some(raw::MatchBinding::Loan(_)))) {
                return Err(reject("generated match loan binding is not implemented yet"));
            }
        }
        raw::Terminator::Return { cleanup, .. } | raw::Terminator::Trap { cleanup, .. } => {
            let plan = f
                .cleanup_plans
                .get(*cleanup as usize)
                .ok_or_else(|| reject("terminator references a foreign cleanup plan"))?;
            budget::charge(
                &mut work,
                plan.actions
                    .len()
                    .checked_add(1)
                    .ok_or_else(|| resource_failure("terminator cleanup scan overflow"))?,
            )?;
            if plan.actions.iter().any(|action| matches!(action, raw::CleanupAction::EndLoan(_))) {
                return Err(reject(
                    "terminator loan cleanup requires a retained source-bound use fact",
                ));
            }
        }
        raw::Terminator::WeakUpgradeBranch { .. } => {
            return Err(reject("Weak upgrade branch state is not implemented yet"));
        }
        raw::Terminator::Jump(_) | raw::Terminator::Branch { .. } => {}
    }
    *spent = work;
    Ok(())
}

pub(in crate::generic_v1::execution_v1) fn role(loan: &raw::Loan) -> Result<(), Failure> {
    match loan.region.role {
        raw::RegionRole::Lexical => {}
        raw::RegionRole::SelectedMatchArm => {
            if !matches!(
                loan.origin,
                raw::LoanOrigin::StaticProjection {
                    projection: raw::StaticProjection::EnumPayload(_),
                    ..
                }
            ) {
                return Err(reject("selected arm role requires an exact enum payload origin"));
            }
        }
        raw::RegionRole::TransientIndexedAccess => {
            if !matches!(
                loan.origin,
                raw::LoanOrigin::Indexed { .. }
                    | raw::LoanOrigin::InternalReborrow(_)
                    | raw::LoanOrigin::StaticProjection { .. }
            ) {
                return Err(reject("transient indexed role requires exact indexed ancestry"));
            }
        }
    }
    match loan.origin {
        raw::LoanOrigin::StaticProjection {
            projection: raw::StaticProjection::EnumPayload(_),
            ..
        } if loan.region.role != raw::RegionRole::SelectedMatchArm => {
            return Err(reject("enum payload loan needs selected arm authority"));
        }
        raw::LoanOrigin::Indexed { .. }
        | raw::LoanOrigin::Formal(_)
        | raw::LoanOrigin::Place(_)
        | raw::LoanOrigin::InternalReborrow(_)
        | raw::LoanOrigin::StaticProjection { .. } => {}
    }
    Ok(())
}
