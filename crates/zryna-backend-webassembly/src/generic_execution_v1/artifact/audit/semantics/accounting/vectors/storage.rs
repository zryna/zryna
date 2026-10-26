//! Old descriptor and header invariants derive only from exact retained element layout.
use super::{Diagnostic, Proof, T};
use zryna_ownership_runtime_abi::generic_v1::ElementLayout;
pub(super) fn valid(
    p: &mut Proof<'_, '_, '_, '_, '_>,
    row: &ElementLayout,
    pointer: i64,
    length: i64,
    capacity: i64,
) -> Result<(), Diagnostic> {
    let ceiling = crate::generic_execution_v1::checked_i64(
        zryna_ownership_runtime_abi::MAX_VEC_ELEMENTS
            .min(zryna_ownership_runtime_abi::MAX_DYNAMIC_ALLOCATION_BYTES / row.stride()),
    )?;
    p.need(&[
        T(1, length),
        T(1, capacity),
        T(16, 0),
        T(1, capacity),
        T(6, ceiling),
        T(16, 0),
        T(18, 0),
        T(21, 0),
    ])?;
    p.fail(255, 255)?;
    p.need(&[T(28, 0), T(1, capacity), T(14, 0), T(21, 0), T(1, pointer), T(21, 0)])?;
    p.fail(255, 255)?;
    p.need(&[
        T(28, 0),
        T(36, 0),
        T(1, pointer),
        T(6, i64::from(p.model.memory.claims().heap_start) + 16),
        T(15, 0),
        T(1, pointer),
        T(4, 0),
        T(17, 0),
        T(18, 0),
        T(1, pointer),
        T(6, 7),
        T(19, 0),
        T(18, 0),
        T(21, 0),
    ])?;
    p.fail(255, 255)?;
    p.need(&[T(28, 0)])?;
    for (offset, value) in
        [(8, 1), (12, crate::generic_execution_v1::checked_i64(row.alignment())?)]
    {
        p.need(&[
            T(1, pointer),
            T(6, 16),
            T(10, 0),
            T(7, offset),
            T(6, value),
            T(13, 0),
            T(21, 0),
        ])?;
        p.fail(255, 255)?;
        p.need(&[T(28, 0)])?;
    }
    p.need(&[
        T(1, pointer),
        T(6, 16),
        T(10, 0),
        T(7, 4),
        T(1, capacity),
        T(6, crate::generic_execution_v1::checked_i64(row.stride())?),
        T(11, 0),
        T(13, 0),
        T(21, 0),
    ])?;
    p.fail(255, 255)?;
    p.need(&[T(28, 0), T(28, 0)])
}
