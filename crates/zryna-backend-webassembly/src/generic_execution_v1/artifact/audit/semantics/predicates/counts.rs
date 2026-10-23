//! Count field guards and the complete size/pending composite at the actual If.
use super::{A, Diagnostic, Guard, Model, Window, release};
use crate::generic_execution_v1::model::containers as C;
pub(super) fn classify(w: &Window, m: &Model<'_, '_, '_>) -> Result<Option<Guard>, Diagnostic> {
    if let Some(guard) = release::common(w, m)? {
        return Ok(Some(guard));
    }
    let pending = w.role == C::STRONG_FINISH;
    let flags = if pending {
        w.matches(
            &[
                A(1, 0),
                A(6, 16),
                A(10, 0),
                A(7, 4),
                A(6, 8),
                A(15, 0),
                A(1, 0),
                A(6, 16),
                A(10, 0),
                A(7, 12),
                A(6, 256),
                A(19, 0),
                A(14, 0),
                A(18, 0),
                A(21, 0),
            ],
            m,
        )?
    } else {
        w.matches(
            &[
                A(1, 0),
                A(6, 16),
                A(10, 0),
                A(7, 4),
                A(6, 8),
                A(15, 0),
                A(1, 0),
                A(6, 16),
                A(10, 0),
                A(7, 12),
                A(6, 256),
                A(19, 0),
                A(18, 0),
                A(21, 0),
            ],
            m,
        )?
    };
    if flags {
        return Ok(Some(Guard::ControlFlags(pending)));
    }
    let zero: Option<u32> = match w.role {
        C::STRONG_CLONE | C::DOWNGRADE | C::STRONG_BEGIN => Some(0),
        C::WEAK_CLONE | C::STRONG_FINISH => Some(4),
        _ => None,
    };
    if let Some(field) = zero
        && w.matches(&[A(1, 0), A(7, i64::from(field)), A(14, 0), A(21, 0)], m)?
    {
        return Ok(Some(Guard::CountZero(field)));
    }
    let maximum: Option<u32> = match w.role {
        C::STRONG_CLONE => Some(0),
        C::DOWNGRADE | C::WEAK_CLONE => Some(4),
        _ => None,
    };
    if let Some(field) = maximum
        && w.matches(&[A(1, 0), A(7, i64::from(field)), A(6, -1), A(12, 0), A(21, 0)], m)?
    {
        return Ok(Some(Guard::CountMax(field)));
    }
    if w.role == C::STRONG_FINISH
        && w.matches(&[A(1, 0), A(7, 0), A(1, 0), A(7, 4), A(14, 0), A(18, 0), A(21, 0)], m)?
    {
        return Ok(Some(Guard::FinishCounts));
    }
    if w.role == C::WEAK_RELEASE
        && w.matches(
            &[A(1, 0), A(7, 4), A(1, 0), A(7, 0), A(14, 0), A(14, 0), A(32, 0), A(21, 0)],
            m,
        )?
    {
        return Ok(Some(Guard::WeakReleaseCounts));
    }
    Ok(None)
}
