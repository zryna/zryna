//! Complete certified Clone/init/rollback SSA and checking work, in addition to unchanged Drop.
use super::super::containers::ContainerPlan;
use super::{Diagnostic, Shape, budget, take};
use zryna_ir::generic_v1::execution_v1::CloneNodeKind;
pub(in crate::generic_execution_v1) fn emission(
    types: &[Shape<'_>],
    plan: &ContainerPlan<'_, '_>,
    work: &mut u64,
) -> Result<(), Diagnostic> {
    let count = u64::try_from(types.len()).map_err(|_| budget())?;
    take(work, count.checked_mul(16).ok_or_else(budget)?)?;
    let mut instructions = 77u64;
    let mut members = 0u64;
    let mut helpers = 0u64;
    for index in 0..types.len() {
        #[cfg(test)]
        visits::row();
        let ty = u32::try_from(index).map_err(|_| budget())?;
        let Some(node) = plan.clone_node(ty) else {
            continue;
        };
        helpers = helpers.checked_add(1).ok_or_else(budget)?;
        let cost = match node.kind() {
            CloneNodeKind::Copy => 30,
            CloneNodeKind::String => 101,
            CloneNodeKind::Struct => 17,
            CloneNodeKind::Enum => 20,
            CloneNodeKind::FixedArray { .. } => 33,
            CloneNodeKind::VecEach => 183,
            CloneNodeKind::SharedCount | CloneNodeKind::WeakCount => 99,
        };
        instructions = instructions.checked_add(cost).ok_or_else(budget)?;
        members = members
            .checked_add(node.fields().len() as u64)
            .and_then(|n| n.checked_add(node.variants().len() as u64))
            .ok_or_else(budget)?;
    }
    instructions = instructions
        .checked_add(members.checked_mul(16).ok_or_else(budget)?)
        .and_then(|n| n.checked_add(helpers.checked_mul(3)?))
        .ok_or_else(budget)?;
    take(work, instructions.checked_mul(128).ok_or_else(budget)?)?;
    take(work, members.checked_mul(16).ok_or_else(budget)?)
}
#[cfg(test)]
mod visits {
    std::thread_local! {static ROWS:std::cell::Cell<u64>=const{std::cell::Cell::new(0)};}
    pub(super) fn row() {
        ROWS.with(|n| n.set(n.get() + 1));
    }
    pub(super) fn reset() {
        ROWS.with(|n| n.set(0));
    }
    pub(super) fn count() -> u64 {
        ROWS.with(std::cell::Cell::get)
    }
}
#[cfg(test)]
mod tests;
