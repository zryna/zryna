//! Exact original Weak/Shared identity independently of any runtime decision.
use super::{Failure, Function, Kind, TypeWorld, budget, raw, reject};
pub(super) fn check(
    f: &raw::Function,
    checked: &Function,
    world: &TypeWorld,
    t: &raw::Terminator,
    spent: &mut usize,
) -> Result<(), Failure> {
    let mut work = *spent;
    let raw::Terminator::WeakUpgradeBranch { weak, success_binding, .. } = t else {
        return Err(reject("upgrade typing requires exact Weak branch opcode"));
    };
    budget::charge(&mut work, 6)?;
    let source = match weak {
        raw::PlaceOrLoan::Place(id) => checked.places.rows.get(*id as usize).map(|p| p.ty),
        raw::PlaceOrLoan::Loan(id) => f.loans.get(*id as usize).map(|l| l.referent),
    }
    .ok_or_else(|| reject("upgrade source is foreign"))?;
    let raw::ValueType::Stored(shared) = success_binding.ty else {
        return Err(reject("upgrade success requires stored Shared binding"));
    };
    if world.kind(source) != Some(Kind::Weak) || world.kind(shared) != Some(Kind::Shared) {
        return Err(reject("upgrade requires exact Weak and Shared original heads"));
    }
    let source = world.arguments(source).ok_or_else(|| reject("Weak argument absent"))?;
    let target = world.arguments(shared).ok_or_else(|| reject("Shared argument absent"))?;
    if source.len() != 1 || target != source {
        return Err(reject("upgrade changes exact original referent identity"));
    }
    *spent = work;
    Ok(())
}
