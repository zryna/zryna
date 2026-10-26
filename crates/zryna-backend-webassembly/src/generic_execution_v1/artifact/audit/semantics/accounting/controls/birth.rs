//! Birth requires H2 before the raw packed call, exact low-zero failures and raw-owned physical L.
use super::{C, Diagnostic, Proof, T, common};
pub(super) fn check(p: &mut Proof<'_, '_, '_, '_, '_>) -> Result<(), Diagnostic> {
    let scratch = i64::from(p.model.memory.claims().scratch_start);
    p.need(&[T(6, scratch), T(39, 0), T(40, 0)])?;
    common::context(p, true)?;
    p.need(&[
        T(1, 0),
        T(14, 0),
        T(14, 0),
        T(1, 0),
        T(6, 2_147_483_647),
        T(32, 0),
        T(19, 0),
        T(1, 1),
        T(14, 0),
        T(14, 0),
        T(19, 0),
        T(1, 1),
        T(6, 8),
        T(32, 0),
        T(19, 0),
        T(1, 1),
        T(1, 1),
        T(6, 1),
        T(10, 0),
        T(19, 0),
        T(14, 0),
        T(19, 0),
        T(3, 3),
        T(6, 2),
        T(11, 0),
        T(2, 2),
        T(4, 8),
        T(2, 6),
    ])?;
    common::admit(p, 2, 3, true)?;
    p.need(&[
        T(6, scratch),
        T(1, 0),
        T(1, 1),
        T(31, i64::from(p.model.container_base + C::ALLOCATE_RAW)),
        T(40, 0),
        T(6, scratch),
        T(7, 4),
        T(2, 4),
        T(6, scratch),
        T(7, 0),
        T(2, 5),
        T(1, 4),
        T(21, 0),
    ])?;
    common::refund(p, 2)?;
    p.need(&[T(1, 5), T(4, 8), T(1, 6), T(13, 0), T(18, 0), T(21, 0)])?;
    common::fail(p, 255, 255, true)?;
    p.need(&[T(28, 0)])?;
    for (index, status) in [1, 2, 255].into_iter().enumerate() {
        p.need(&[T(1, 4), T(6, status), T(12, 0)])?;
        if index > 0 {
            p.need(&[T(18, 0)])?;
        }
    }
    p.need(&[T(14, 0), T(21, 0)])?;
    common::fail(p, 255, 255, true)?;
    p.need(&[
        T(28, 0),
        T(1, 4),
        T(41, 0),
        T(39, 32),
        T(42, 0),
        T(22, 0),
        T(28, 0),
        T(1, 0),
        T(14, 0),
        T(21, 0),
        T(1, 5),
        T(4, 8),
        T(1, 6),
        T(13, 0),
        T(18, 0),
        T(21, 0),
    ])?;
    common::fail(p, 255, 255, true)?;
    p.need(&[
        T(28, 0),
        T(39, 0),
        T(22, 0),
        T(28, 0),
        T(1, 3),
        T(14, 0),
        T(1, 5),
        T(14, 0),
        T(18, 0),
        T(4, 8),
        T(1, 6),
        T(6, 1),
        T(9, 0),
        T(13, 0),
        T(18, 0),
        T(21, 0),
    ])?;
    common::fail(p, 255, 255, true)?;
    p.need(&[T(28, 0), T(1, 5), T(41, 0)])
}
