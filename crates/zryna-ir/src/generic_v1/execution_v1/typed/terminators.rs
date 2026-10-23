use super::{Failure, Function, Kind, TypeWorld, budget, operations, raw, reject};
mod upgrade;

pub(super) fn check(
    f: &raw::Function,
    checked: &Function,
    world: &TypeWorld,
    bi: usize,
    work: &mut usize,
) -> Result<(), Failure> {
    let block = &f.blocks[bi];
    if !matches!(block.terminator, raw::Terminator::WeakUpgradeBranch { .. }) {
        super::loan_sites::terminator(f, &block.terminator, work)?;
    }
    match &block.terminator {
        raw::Terminator::Return { value, .. } => {
            let actual = value
                .map(|id| checked.graph.operand(id, bi, block.instructions.len()))
                .transpose()?
                .unwrap_or(raw::ValueType::Unit);
            if actual != f.result_type {
                return Err(reject("return differs from exact private result type"));
            }
        }
        raw::Terminator::Jump(_) | raw::Terminator::Trap { .. } => {}
        raw::Terminator::Branch { condition, .. } => operations::exact(
            checked.graph.operand(*condition, bi, block.instructions.len())?,
            operations::primitive(world, Kind::Bool, work)?,
        )?,
        raw::Terminator::ClosedEnumMatch { ty, scrutinee, mode, arms } => {
            if !matches!(world.kind(*ty), Some(Kind::Enum { .. } | Kind::Option | Kind::Result)) {
                return Err(reject("match requires an exact enum/family identity"));
            }
            let actual = match scrutinee {
                raw::PlaceOrLoan::Place(id) => checked.places.rows.get(*id as usize).map(|p| p.ty),
                raw::PlaceOrLoan::Loan(id) => f.loans.get(*id as usize).map(|l| l.referent),
            };
            if actual != Some(*ty) {
                return Err(reject("match source type differs from full enum identity"));
            }
            let members = world.members(*ty).ok_or_else(|| reject("match variant proof absent"))?;
            budget::charge(work, members.len())?;
            if arms.len() != members.len() {
                return Err(reject("match does not cover every exact variant"));
            }
            if *mode != raw::MatchMode::Value {
                return Err(reject("borrowed payload match transitions are not implemented yet"));
            }
            for (ordinal, (arm, payload)) in arms.iter().zip(members).enumerate() {
                if arm.ordinal as usize != ordinal {
                    return Err(reject("match ordinal order is duplicated or foreign"));
                }
                match (&arm.binding, payload) {
                    (None, None) => {}
                    (Some(raw::MatchBinding::Value(d)), Some(ty))
                        if d.ty == raw::ValueType::Stored(*ty) => {}
                    _ => {
                        return Err(reject(
                            "match generated binding differs from exact active payload",
                        ));
                    }
                }
            }
        }
        raw::Terminator::WeakUpgradeBranch { .. } => {
            upgrade::check(f, checked, world, &block.terminator, work)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod upgrade_tests;
