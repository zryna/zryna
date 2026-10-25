use super::{Failure, Function, Kind, TypeWorld, budget, operations, raw, reject};
mod matches;
mod upgrade;

pub(super) fn check(
    f: &raw::Function,
    checked: &Function,
    world: &TypeWorld,
    bi: usize,
    work: &mut usize,
) -> Result<(), Failure> {
    let block = &f.blocks[bi];
    if !matches!(
        block.terminator,
        raw::Terminator::WeakUpgradeBranch { .. }
            | raw::Terminator::ClosedEnumMatch {
                mode: raw::MatchMode::SharedLoan | raw::MatchMode::ExclusiveLoan,
                ..
            }
    ) {
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
        raw::Terminator::ClosedEnumMatch { .. } => {
            matches::check(f, checked, world, &block.terminator, work)?;
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

#[cfg(test)]
mod borrowed_tests;
