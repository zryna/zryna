//! Exactly one selected-edge issuance, never an ordinary instruction or invented arena authority.
use super::{Failure, budget, raw, reject, reserve, resource_failure};
pub(super) fn check(f: &raw::Function, spent: &mut usize) -> Result<(), Failure> {
    let mut work = *spent;
    budget::charge(
        &mut work,
        f.loans
            .len()
            .checked_mul(2)
            .and_then(|n| n.checked_add(f.blocks.len()))
            .ok_or_else(|| resource_failure("selected loan census overflow"))?,
    )?;
    let mut issued = reserve(f.loans.len())?;
    issued.resize(f.loans.len(), false);
    for block in &f.blocks {
        budget::charge(&mut work, block.instructions.len())?;
        for i in &block.instructions {
            if let raw::Operation::BeginLoan { loan, .. } = i.operation
                && f.loans
                    .get(loan as usize)
                    .is_some_and(|l| l.region.role == raw::RegionRole::SelectedMatchArm)
            {
                return Err(reject("selected arm loan cannot begin at an ordinary instruction"));
            }
        }
        if let raw::Terminator::ClosedEnumMatch { scrutinee, mode, arms, .. } = &block.terminator {
            budget::charge(
                &mut work,
                arms.len()
                    .checked_mul(4)
                    .ok_or_else(|| resource_failure("selected arm work overflow"))?,
            )?;
            for arm in arms {
                if let Some(raw::MatchBinding::Loan(d)) = &arm.binding {
                    let raw::PlaceOrLoan::Loan(parent) = scrutinee else {
                        return Err(reject("selected child requires exact loan scrutinee"));
                    };
                    let access = match mode {
                        raw::MatchMode::SharedLoan => raw::Access::Shared,
                        raw::MatchMode::ExclusiveLoan => raw::Access::Exclusive,
                        raw::MatchMode::Value => {
                            return Err(reject("owned match cannot issue a selected loan"));
                        }
                    };
                    let loan = f
                        .loans
                        .get(d.id as usize)
                        .ok_or_else(|| reject("generated selected loan is foreign"))?;
                    let slot = issued.get_mut(d.id as usize).ok_or(Failure::InternalFailure)?;
                    if *slot
                        || loan.id != d.id
                        || loan.referent != d.referent
                        || loan.access != d.access
                        || d.access != access
                        || loan.region.role != raw::RegionRole::SelectedMatchArm
                        || loan.origin
                            != (raw::LoanOrigin::StaticProjection {
                                parent: *parent,
                                projection: raw::StaticProjection::EnumPayload(arm.ordinal),
                            })
                    {
                        return Err(reject(
                            "selected loan differs from its unique exact parent/ordinal/binding",
                        ));
                    }
                    *slot = true;
                }
            }
        }
    }
    for (loan, issued) in f.loans.iter().zip(issued) {
        if (loan.region.role == raw::RegionRole::SelectedMatchArm) != issued {
            return Err(reject("selected loan has missing or foreign edge issuance"));
        }
    }
    *spent = work;
    Ok(())
}
