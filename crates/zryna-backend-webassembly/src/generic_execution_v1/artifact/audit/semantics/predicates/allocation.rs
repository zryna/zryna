//! Canonical allocation argument, fault, size and checked-address guard identities.
use super::{A, Diagnostic, Guard, Model, Window};
pub(super) fn classify(w: &Window, m: &Model<'_, '_, '_>) -> Result<Option<Guard>, Diagnostic> {
    if w.matches(
        &[
            A(1, 1),
            A(14, 0),
            A(1, 1),
            A(6, 8),
            A(16, 0),
            A(18, 0),
            A(1, 1),
            A(1, 1),
            A(6, 1),
            A(10, 0),
            A(19, 0),
            A(18, 0),
            A(21, 0),
        ],
        m,
    )? {
        return Ok(Some(Guard::AllocateArguments));
    }
    if w.matches(&[A(1, 0), A(14, 0), A(21, 0)], m)? {
        return Ok(Some(Guard::AllocateZero));
    }
    if m.observed()
        && w.matches(
            &[A(4, 3), A(6, 2), A(12, 0), A(4, 5), A(4, 4), A(12, 0), A(19, 0), A(21, 0)],
            m,
        )?
    {
        return Ok(Some(Guard::AllocationFault));
    }
    let capacity = crate::generic_execution_v1::checked_i64(
        zryna_ownership_runtime_abi::MAX_DYNAMIC_ALLOCATION_BYTES - 16,
    )?;
    if w.matches(&[A(1, 0), A(6, capacity), A(16, 0), A(21, 0)], m)? {
        return Ok(Some(Guard::AllocationCapacity));
    }
    if w.matches(
        &[
            A(4, 0),
            A(3, 2),
            A(6, 16),
            A(9, 0),
            A(3, 3),
            A(1, 0),
            A(9, 0),
            A(6, 7),
            A(9, 0),
            A(6, -8),
            A(19, 0),
            A(3, 4),
            A(1, 2),
            A(15, 0),
            A(1, 4),
            A(6, i64::from(crate::generic_execution_v1::memory::MEMORY_BYTES)),
            A(16, 0),
            A(18, 0),
            A(21, 0),
        ],
        m,
    )? {
        return Ok(Some(Guard::AllocationAddress));
    }
    Ok(None)
}
