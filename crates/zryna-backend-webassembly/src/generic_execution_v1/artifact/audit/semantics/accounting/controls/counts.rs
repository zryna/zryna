//! Known count failures are guarded by fresh exact fields and retain H0 admission.
use super::{C, Diagnostic, Proof, T, common};
pub(super) fn check(p: &mut Proof<'_, '_, '_, '_, '_>, offset: usize) -> Result<(), Diagnostic> {
    let role = match offset {
        1 => C::STRONG_CLONE,
        2 => C::DOWNGRADE,
        3 => C::WEAK_CLONE,
        7 => C::WEAK_UPGRADE,
        _ => return Err(super::super::invariant()),
    };
    common::context(p, false)?;
    common::control(p, false)?;
    common::funded(p)?;
    let field = if role == C::DOWNGRADE || role == C::WEAK_CLONE { 4 } else { 0 };
    let required = if role == C::WEAK_CLONE || role == C::WEAK_UPGRADE { 4 } else { 0 };
    p.need(&[T(1, 0), T(7, required), T(14, 0), T(21, 0)])?;
    p.fail(255, 255)?;
    p.need(&[T(28, 0), T(1, 0), T(7, field), T(6, -1), T(13, 0)])?;
    if role == C::WEAK_UPGRADE {
        p.need(&[T(1, 0), T(7, 0), T(14, 0), T(14, 0), T(19, 0)])?;
    }
    p.need(&[T(2, 1), T(6, 0), T(2, 2), T(4, 8), T(2, 3)])?;
    common::admit(p, 1, 2, false)?;
    p.need(&[
        T(1, 0),
        T(31, i64::from(p.model.container_base + role)),
        T(2, 4),
        T(4, 8),
        T(1, 3),
        T(13, 0),
        T(21, 0),
    ])?;
    p.fail(255, 255)?;
    p.need(&[T(28, 0), T(1, 4), T(21, 0)])?;
    common::refund(p, 1)?;
    for (index, status) in [3, 255].into_iter().enumerate() {
        p.need(&[T(1, 4), T(6, status), T(12, 0)])?;
        if index > 0 {
            p.need(&[T(18, 0)])?;
        }
    }
    if role == C::WEAK_UPGRADE {
        p.need(&[T(1, 4), T(6, 5), T(12, 0), T(18, 0)])?;
    }
    p.need(&[T(14, 0), T(21, 0)])?;
    p.fail(255, 255)?;
    p.need(&[T(28, 0), T(1, 4), T(22, 0), T(28, 0), T(1, 1), T(14, 0), T(21, 0)])?;
    p.fail(255, 255)?;
    p.need(&[T(28, 0), T(6, 0)])
}
