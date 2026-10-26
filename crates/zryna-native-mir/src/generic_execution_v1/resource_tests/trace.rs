//! Test-only observation of actual owning debits/reservations; no authority constructors.
use super::super::{planning::CopySite, raw};
use std::cell::RefCell;
#[derive(Default)]
pub(super) struct Trace {
    pub reserves: usize,
    pub copies: Vec<(CopySite, raw::ValueType, u64)>,
}
thread_local! { static TRACE: RefCell<Option<Trace>> = const { RefCell::new(None) }; }
pub(in super::super) fn reserve(count: usize) {
    TRACE.with_borrow_mut(|slot| {
        if let Some(trace) = slot {
            trace.reserves += usize::from(count != 0);
        }
    });
}
pub(in super::super) fn copy(site: CopySite, ty: raw::ValueType, paid: u64) {
    TRACE.with_borrow_mut(|slot| {
        if let Some(trace) = slot {
            trace.copies.push((site, ty, paid));
        }
    });
}
pub(super) fn start() {
    TRACE.with_borrow_mut(|slot| {
        assert!(slot.is_none(), "non-nested private observation");
        *slot = Some(Trace::default());
    });
}
pub(super) fn finish() -> Trace {
    TRACE.with_borrow_mut(|slot| slot.take().expect("active observation"))
}
