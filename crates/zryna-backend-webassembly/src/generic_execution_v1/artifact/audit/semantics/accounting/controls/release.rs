//! Exact held spends and pre-free predicates bind flags and kernel-owned live transitions.
use super::{C, Diagnostic, Proof, T, common};
pub(super) fn check(p: &mut Proof<'_, '_, '_, '_, '_>, offset: usize) -> Result<(), Diagnostic> {
    let role = match offset {
        4 => C::STRONG_BEGIN,
        5 => C::STRONG_FINISH,
        6 => C::WEAK_RELEASE,
        _ => return Err(super::super::invariant()),
    };
    let parameters = if role == C::STRONG_FINISH { 1 } else { 2 };
    let predicate = parameters;
    let old_live = parameters + 1;
    let status = parameters + 2;
    if parameters == 2 {
        p.need(&[T(1, 1), T(6, 0), T(8, 0)])?;
    }
    common::context(p, false)?;
    common::control(p, role == C::STRONG_FINISH)?;
    common::funded(p)?;
    if role == C::STRONG_BEGIN {
        p.need(&[T(1, 0), T(7, 0), T(14, 0)])?;
    } else if role == C::STRONG_FINISH {
        p.need(&[T(1, 0), T(7, 0), T(1, 0), T(7, 4), T(14, 0), T(18, 0)])?;
    } else {
        p.need(&[T(1, 0), T(7, 4), T(1, 0), T(7, 0), T(14, 0), T(14, 0), T(32, 0)])?;
    }
    p.need(&[T(21, 0)])?;
    p.fail(255, 255)?;
    p.need(&[
        T(28, 0),
        T(1, 0),
        T(7, if role == C::STRONG_BEGIN { 0 } else { 4 }),
        T(6, 1),
        T(12, 0),
    ])?;
    if role == C::WEAK_RELEASE {
        p.need(&[T(1, 0), T(7, 0), T(14, 0), T(19, 0)])?;
    }
    p.need(&[
        T(2, predicate),
        T(4, 8),
        T(2, old_live),
        T(4, 12),
        T(6, 1),
        T(10, 0),
        T(5, 12),
        T(4, 11),
        T(6, 1),
        T(9, 0),
        T(5, 11),
        T(1, 0),
    ])?;
    if parameters == 2 {
        p.need(&[T(1, 1)])?;
    }
    p.need(&[
        T(31, i64::from(p.model.container_base + role)),
        T(2, status),
        T(1, status),
        T(21, 0),
    ])?;
    p.fail(255, 255)?;
    p.need(&[T(28, 0)])?;
    if parameters == 2 {
        p.need(&[T(1, 1), T(7, 0), T(1, predicate), T(13, 0), T(21, 0)])?;
        p.fail(255, 255)?;
        p.need(&[T(28, 0)])?;
    }
    p.need(&[T(4, 8), T(1, old_live)])?;
    if role != C::STRONG_BEGIN {
        p.need(&[T(1, predicate), T(10, 0)])?;
    }
    p.need(&[T(13, 0), T(21, 0)])?;
    p.fail(255, 255)?;
    p.need(&[T(28, 0), T(6, 0)])
}
