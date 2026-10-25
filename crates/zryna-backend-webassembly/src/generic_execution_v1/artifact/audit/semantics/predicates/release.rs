//! Shared pointer/header predicates and role-specific exact raw release arguments.
use super::{A, Diagnostic, Guard, Model, Window};
pub(super) fn common(w: &Window, m: &Model<'_, '_, '_>) -> Result<Option<Guard>, Diagnostic> {
    let first = i64::from(m.memory.claims().heap_start) + 16;
    if w.matches(
        &[
            A(1, 0),
            A(6, first),
            A(15, 0),
            A(1, 0),
            A(4, 0),
            A(17, 0),
            A(18, 0),
            A(1, 0),
            A(6, 7),
            A(19, 0),
            A(18, 0),
            A(21, 0),
        ],
        m,
    )? {
        return Ok(Some(Guard::PointerValidity));
    }
    if w.matches(&[A(1, 0), A(6, 16), A(10, 0), A(7, 8), A(6, 1), A(13, 0), A(21, 0)], m)? {
        return Ok(Some(Guard::HeaderLive));
    }
    Ok(None)
}
pub(super) fn classify(w: &Window, m: &Model<'_, '_, '_>) -> Result<Option<Guard>, Diagnostic> {
    if let Some(guard) = common(w, m)? {
        return Ok(Some(guard));
    }
    if w.matches(&[A(1, 0), A(14, 0), A(21, 0)], m)? {
        return Ok(Some(Guard::ReleaseNull));
    }
    if w.matches(&[A(1, 1), A(21, 0)], m)? {
        return Ok(Some(Guard::ReleaseSize));
    }
    if w.matches(&[A(1, 0), A(6, 16), A(10, 0), A(7, 4), A(1, 1), A(13, 0), A(21, 0)], m)? {
        return Ok(Some(Guard::HeaderSize));
    }
    if w.matches(&[A(1, 0), A(6, 16), A(10, 0), A(7, 12), A(1, 2), A(13, 0), A(21, 0)], m)? {
        return Ok(Some(Guard::HeaderAlignment));
    }
    Ok(None)
}
