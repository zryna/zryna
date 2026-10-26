//! Independent fixed geometry, counter and debit predicates; not an encoder body comparison.
use super::{Diagnostic, Proof, T};
pub(super) fn fail(
    p: &mut Proof<'_, '_, '_, '_, '_>,
    source: i64,
    status: i64,
    wide: bool,
) -> Result<(), Diagnostic> {
    p.need(&[T(6, source), T(5, 1)])?;
    p.need(&[if wide { T(39, status << 32) } else { T(6, status) }, T(22, 0)])
}
pub(super) fn context(p: &mut Proof<'_, '_, '_, '_, '_>, wide: bool) -> Result<(), Diagnostic> {
    p.need(&[
        T(4, 10),
        T(6, 1_048_576),
        T(16, 0),
        T(4, 8),
        T(4, 10),
        T(16, 0),
        T(18, 0),
        T(4, 8),
        T(6, 1_048_576),
        T(16, 0),
        T(18, 0),
        T(4, 11),
        T(6, 4_194_304),
        T(16, 0),
        T(18, 0),
        T(4, 12),
        T(6, 4_194_304),
        T(16, 0),
        T(18, 0),
        T(21, 0),
    ])?;
    fail(p, 255, 255, wide)?;
    p.need(&[T(28, 0), T(4, 11), T(4, 12), T(9, 0), T(6, 4_194_304), T(16, 0), T(21, 0)])?;
    fail(p, 255, 255, wide)?;
    p.need(&[T(28, 0)])
}
pub(super) fn control(p: &mut Proof<'_, '_, '_, '_, '_>, pending: bool) -> Result<(), Diagnostic> {
    let lower = i64::from(p.model.memory.claims().heap_start) + 16;
    p.need(&[
        T(1, 0),
        T(6, lower),
        T(15, 0),
        T(1, 0),
        T(4, 0),
        T(17, 0),
        T(18, 0),
        T(1, 0),
        T(6, 7),
        T(19, 0),
        T(18, 0),
        T(21, 0),
    ])?;
    p.fail(255, 255)?;
    p.need(&[T(28, 0), T(1, 0), T(6, 16), T(10, 0), T(7, 8), T(6, 1), T(13, 0), T(21, 0)])?;
    p.fail(255, 255)?;
    p.need(&[
        T(28, 0),
        T(1, 0),
        T(6, 16),
        T(10, 0),
        T(7, 4),
        T(6, 8),
        T(15, 0),
        T(1, 0),
        T(6, 16),
        T(10, 0),
        T(7, 12),
        T(6, 256),
        T(19, 0),
    ])?;
    if pending {
        p.need(&[T(14, 0)])?;
    }
    p.need(&[T(18, 0), T(21, 0)])?;
    p.fail(255, 255)?;
    p.need(&[T(28, 0)])
}
pub(super) fn funded(p: &mut Proof<'_, '_, '_, '_, '_>) -> Result<(), Diagnostic> {
    p.need(&[T(4, 12), T(14, 0), T(4, 8), T(14, 0), T(18, 0), T(21, 0)])?;
    p.fail(255, 255)?;
    p.need(&[T(28, 0)])
}
pub(super) fn admit(
    p: &mut Proof<'_, '_, '_, '_, '_>,
    hold: i64,
    positive: i64,
    wide: bool,
) -> Result<(), Diagnostic> {
    p.need(&[
        T(4, 11),
        T(4, 12),
        T(9, 0),
        T(6, 1),
        T(9, 0),
        T(1, hold),
        T(9, 0),
        T(6, 4_194_304),
        T(16, 0),
        T(4, 10),
        T(1, positive),
        T(9, 0),
        T(6, 1_048_576),
        T(16, 0),
        T(18, 0),
        T(4, 8),
        T(1, positive),
        T(9, 0),
        T(6, 1_048_576),
        T(16, 0),
        T(18, 0),
        T(21, 0),
    ])?;
    fail(p, 3, 2, wide)?;
    p.need(&[
        T(28, 0),
        T(4, 11),
        T(6, 1),
        T(9, 0),
        T(5, 11),
        T(4, 10),
        T(1, positive),
        T(9, 0),
        T(5, 10),
        T(4, 12),
        T(1, hold),
        T(9, 0),
        T(5, 12),
    ])
}
pub(super) fn refund(p: &mut Proof<'_, '_, '_, '_, '_>, hold: i64) -> Result<(), Diagnostic> {
    p.need(&[T(4, 12), T(1, hold), T(10, 0), T(5, 12)])
}
