//! Role-derived whole adapter grammar: guard dominance, exact debit/rollback and canonical call.
use super::{Diagnostic, Proof, T};
pub(super) fn check(p: &mut Proof<'_, '_, '_, '_, '_>, role: usize) -> Result<(), Diagnostic> {
    if role == 2 {
        return release(p);
    }
    let parameters = if role == 1 { 4 } else { 3 };
    let possible = parameters + 1;
    let positive = parameters + 2;
    let status = parameters + 3;
    prepare(p, role, parameters)?;
    let out = if role == 1 { 3 } else { 2 };
    p.need(&[
        T(4, 11),
        T(4, 12),
        T(9, 0),
        T(6, 1),
        T(9, 0),
        T(1, possible),
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
    p.fail(3, 2)?;
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
        T(1, possible),
        T(9, 0),
        T(5, 12),
    ])?;
    for parameter in 0..parameters {
        p.need(&[T(1, parameter)])?;
    }
    let target = if role == 0 {
        1
    } else if role == 1 {
        2
    } else {
        i64::from(p.model.concat_index())
    };
    p.need(&[
        T(31, target),
        T(2, status),
        T(1, status),
        T(21, 0),
        T(4, 12),
        T(1, possible),
        T(10, 0),
        T(5, 12),
    ])?;
    p.zero(out)?;
    p.need(&[
        T(1, status),
        T(6, 1),
        T(12, 0),
        T(1, status),
        T(6, 2),
        T(12, 0),
        T(18, 0),
        T(1, status),
        T(6, 255),
        T(12, 0),
        T(18, 0),
    ])?;
    if role == 0 {
        p.need(&[T(1, status), T(6, 4), T(12, 0), T(18, 0)])?;
    }
    p.need(&[T(14, 0), T(21, 0)])?;
    p.fail(255, 255)?;
    p.need(&[T(28, 0), T(1, status), T(22, 0), T(28, 0), T(1, possible), T(14, 0), T(21, 0)])?;
    p.fail(255, 255)?;
    p.need(&[T(28, 0), T(6, 0)])
}
fn prepare(
    p: &mut Proof<'_, '_, '_, '_, '_>,
    role: usize,
    parameters: i64,
) -> Result<(), Diagnostic> {
    let length = parameters;
    let possible = parameters + 1;
    let positive = parameters + 2;
    if role == 3 {
        p.need(&[
            T(1, 0),
            T(7, 4),
            T(2, 7),
            T(1, 1),
            T(7, 4),
            T(1, 7),
            T(9, 0),
            T(3, length),
            T(1, 7),
            T(17, 0),
            T(1, length),
            T(6, 2_147_483_647),
            T(32, 0),
            T(19, 0),
            T(2, possible),
            T(1, 1),
            T(7, 4),
            T(1, 7),
            T(18, 0),
            T(2, 7),
        ])?;
    } else {
        p.need(&[T(1, 1), T(3, length), T(6, 2_147_483_647), T(32, 0), T(2, possible)])?;
    }
    p.need(&[T(1, length), T(14, 0), T(14, 0), T(1, possible), T(19, 0), T(2, positive)])?;
    let out = if role == 1 { 3 } else { 2 };
    p.zero(out)?;
    p.context()?;
    if role == 1 || role == 3 {
        p.funded(if role == 3 { 7 } else { length })?;
    }
    Ok(())
}
fn release(p: &mut Proof<'_, '_, '_, '_, '_>) -> Result<(), Diagnostic> {
    p.context()?;
    p.funded(1)?;
    p.need(&[
        T(4, 12),
        T(6, 1),
        T(10, 0),
        T(5, 12),
        T(4, 11),
        T(6, 1),
        T(9, 0),
        T(5, 11),
        T(1, 0),
        T(1, 1),
        T(1, 2),
        T(31, 3),
        T(2, 3),
        T(1, 3),
        T(21, 0),
    ])?;
    p.fail(255, 255)?;
    p.need(&[T(28, 0), T(6, 0)])
}
