//! The exact original match span comes only from independently authenticated source replay.
use super::{Failure, at, budget, raw, reject};
use zryna_source::UntrustedSpan;
pub(in crate::generic_v1::execution_v1::typed) fn check(
    f: &raw::Function,
    t: &raw::Terminator,
    site: UntrustedSpan,
    spent: &mut usize,
) -> Result<(), Failure> {
    let mut work = *spent;
    let raw::Terminator::ClosedEnumMatch {
        scrutinee: raw::PlaceOrLoan::Loan(parent),
        mode,
        arms,
        ..
    } = t
    else {
        return Err(reject("borrowed match site requires exact loan terminator"));
    };
    let loan = at(f, *parent, site, &mut work)?;
    let access = match mode {
        raw::MatchMode::SharedLoan => raw::Access::Shared,
        raw::MatchMode::ExclusiveLoan => raw::Access::Exclusive,
        raw::MatchMode::Value => {
            return Err(reject("owned match cannot consume loan source authority"));
        }
    };
    if loan.access != access {
        return Err(reject("borrowed match changes exact scrutinee access"));
    }
    budget::charge(&mut work, arms.len())?;
    for arm in arms {
        if let Some(raw::MatchBinding::Loan(d)) = &arm.binding {
            let child =
                f.loans.get(d.id as usize).ok_or_else(|| reject("selected arm loan is foreign"))?;
            if !crate::generic_v1::execution_v1::places::within(site, child.region.span) {
                return Err(reject("selected arm region is outside exact original match"));
            }
        }
    }
    *spent = work;
    Ok(())
}
