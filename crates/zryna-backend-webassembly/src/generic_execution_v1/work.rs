//! Existing Wasm generic emission/index work bound, distinct from artifact and address bounds.
//! Each traversal visit costs one unit; retained vector slots cost their Rust byte size,
//! before reserve/write. Literal copies/comparisons cost byte length. This is a private
//! stage ledger, not a transitive counter for verified IR/layout/runtime/library work.
use super::{Diagnostic, MAX_BYTES, budget};
pub(super) fn take(work: &mut usize, units: usize) -> Result<(), Diagnostic> {
    let next = work.checked_add(units).filter(|n| *n <= MAX_BYTES).ok_or_else(budget)?;
    *work = next;
    Ok(())
}
pub(super) fn retained<T>(work: &mut usize, count: usize) -> Result<(), Diagnostic> {
    take(work, count.checked_mul(size_of::<T>()).ok_or_else(budget)?)
}

#[cfg(test)]
thread_local! {static RESERVES:std::cell::Cell<usize>=const{std::cell::Cell::new(0)};}
#[cfg(test)]
pub(super) fn reserve_seen(count: usize) {
    if count != 0 {
        RESERVES.with(|n| n.set(n.get() + 1));
    }
}
#[cfg(test)]
pub(super) fn reset_seen() {
    RESERVES.with(|n| n.set(0));
}
#[cfg(test)]
pub(super) fn seen() -> usize {
    RESERVES.with(std::cell::Cell::get)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_work_boundary_and_first_extra_are_independent_of_output_bytes() {
        let mut work = MAX_BYTES - 1;
        take(&mut work, 1).expect("fixed genuine source/target control");
        assert_eq!(work, MAX_BYTES);
        assert!(take(&mut work, 1).is_err());
        assert_eq!(work, MAX_BYTES);
        let mut work = MAX_BYTES - size_of::<u64>();
        retained::<u64>(&mut work, 1).expect("fixed genuine source/target control");
        assert!(retained::<u64>(&mut work, 1).is_err());
    }
}
