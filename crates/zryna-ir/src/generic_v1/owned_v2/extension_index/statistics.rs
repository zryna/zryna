//! Private per-thread observations at actual index builder/reserve/commit sites.
use std::cell::Cell;

thread_local! {
    static INCOMING: Cell<Option<usize>> = const { Cell::new(None) };
    static COMMITTED: Cell<Option<usize>> = const { Cell::new(None) };
    static RESERVES: Cell<usize> = const { Cell::new(0) };
}

pub(in super::super) fn reset() {
    INCOMING.set(None);
    COMMITTED.set(None);
    RESERVES.set(0);
}

pub(super) fn begin(used: usize) {
    INCOMING.set(Some(used));
    COMMITTED.set(None);
}

pub(super) fn reserve_attempt() {
    RESERVES.set(RESERVES.get().checked_add(1).expect("bounded table reserve count"));
}

pub(super) fn commit(used: usize) {
    COMMITTED.set(Some(used));
}

pub(in super::super) fn observed() -> (Option<usize>, Option<usize>, usize) {
    (INCOMING.get(), COMMITTED.get(), RESERVES.get())
}
