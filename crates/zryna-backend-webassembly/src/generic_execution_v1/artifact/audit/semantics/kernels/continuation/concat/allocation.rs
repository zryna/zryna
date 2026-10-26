//! Private raw allocation obligations bind canonical versus target capacity and exact unpublished packed result.
use super::{A, Diagnostic, Proof};
fn fail(p: &mut Proof<'_, '_, '_, '_>, category: i64, status: i64) -> Result<(), Diagnostic> {
    p.add(&[A(6, category), A(5, 1), A(33, status << 32), A(22, 0)])
}
pub(in crate::generic_execution_v1::artifact::audit::semantics::kernels::continuation) fn allocator(
    p: &mut Proof<'_, '_, '_, '_>,
) -> Result<(), Diagnostic> {
    p.add(&[
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
    ])?;
    fail(p, 255, 255)?;
    p.add(&[
        A(28, 0),
        A(1, 0),
        A(14, 0),
        A(21, 0),
        A(33, 0),
        A(22, 0),
        A(28, 0),
        A(4, 5),
        A(6, 1),
        A(9, 0),
        A(5, 5),
    ])?;
    if p.model.observed() {
        p.add(&[A(4, 3), A(6, 2), A(12, 0), A(4, 5), A(4, 4), A(12, 0), A(19, 0), A(21, 0)])?;
        fail(p, 2, 1)?;
        p.add(&[A(28, 0)])?;
    }
    p.add(&[
        A(1, 0),
        A(
            6,
            crate::generic_execution_v1::checked_i64(
                zryna_ownership_runtime_abi::MAX_DYNAMIC_ALLOCATION_BYTES,
            )?,
        ),
        A(16, 0),
        A(21, 0),
    ])?;
    fail(p, 3, 2)?;
    p.add(&[
        A(28, 0),
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
    ])?;
    fail(p, 2, 1)?;
    p.add(&[
        A(28, 0),
        A(1, 4),
        A(5, 0),
        A(4, 7),
        A(6, 1),
        A(9, 0),
        A(5, 7),
        A(4, 8),
        A(6, 1),
        A(9, 0),
        A(5, 8),
    ])?;
    for (offset, value) in [(0, A(4, 7)), (4, A(1, 0)), (8, A(6, 1)), (12, A(1, 1))] {
        p.add(&[A(1, 2), value, A(8, offset)])?;
    }
    p.add(&[A(1, 3), A(34, 0)])
}
