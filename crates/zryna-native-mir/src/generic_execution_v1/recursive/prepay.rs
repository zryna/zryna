//! Upfront finite Drop/finder/lifecycle SSA, audit and compact metadata scratch costs.
use super::{Diagnostic, Shape, budget, take};
use zryna_ir::generic_v1::execution_v1::VerifiedExecution;
use zryna_layout::TypeCategory;
pub(in crate::generic_execution_v1) fn emission(
    program: &VerifiedExecution<'_>,
    types: &[Shape<'_>],
    work: &mut u64,
) -> Result<(), Diagnostic> {
    let count = u64::try_from(types.len()).map_err(|_| budget())?;
    take(work, count.checked_mul(16).ok_or_else(budget)?)?;
    let mut edges = 0u64;
    let mut cases = 0u64;
    let mut drops = 0u64;
    for shape in types {
        #[cfg(test)]
        visits::row();
        cases = cases.checked_add(case_instructions(shape.view.category())).ok_or_else(budget)?;
        edges = edges
            .checked_add(u64::try_from(shape.view.fields().len()).map_err(|_| budget())?)
            .and_then(|n| n.checked_add(shape.view.variants().len() as u64))
            .ok_or_else(budget)?;
        drops += u64::from(shape.view.drop_kind() != 0);
    }
    let exports = u64::try_from(program.scalar_abi().exports().len()).map_err(|_| budget())?;
    // Complete category recipes fund all four private functions. Fixed/member,
    // wrapper/forwarder terms and128 units per instruction remain unchanged.
    let instructions = cases
        .checked_add(edges.checked_mul(64).ok_or_else(budget)?)
        .and_then(|n| n.checked_add(exports.checked_mul(96)?))
        .and_then(|n| n.checked_add(drops.checked_mul(12)?))
        .and_then(|n| n.checked_add(192))
        .ok_or_else(budget)?;
    take(work, instructions.checked_mul(128).ok_or_else(budget)?)?;
    // Compact Struct/member scratch consists only of certified u32 IDs/offsets.
    take(work, edges.checked_mul(16).ok_or_else(budget)?)
}

// Counts are derived from the independently matched complete primitive recipes.
// Struct fields/Enum variants have their separate unchanged64-instruction term.
fn case_instructions(category: TypeCategory) -> u64 {
    match category {
        TypeCategory::Bool | TypeCategory::I32 | TypeCategory::Enum => 5,
        TypeCategory::String => 58,
        TypeCategory::Struct => 10,
        TypeCategory::FixedArray => 116,
        TypeCategory::Vec => 133,
        TypeCategory::Shared => 144,
        TypeCategory::Weak => 71,
    }
}

#[cfg(test)]
mod visits {
    std::thread_local! {static ROWS: std::cell::Cell<u64> = const {std::cell::Cell::new(0)};}
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
