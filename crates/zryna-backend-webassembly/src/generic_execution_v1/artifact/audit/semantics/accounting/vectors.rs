//! Exact decoded Vec3 accounting obligations; input identities/layouts come from the retained world.
use super::{Diagnostic, Proof, T, invariant};
use crate::generic_execution_v1::model::containers as C;
use zryna_layout::StorageTarget;
mod storage;
pub(super) fn check(p: &mut Proof<'_, '_, '_, '_, '_>, role: usize) -> Result<(), Diagnostic> {
    let m = p.model;
    match role {
        0 => {
            p.zero(2)?;
            p.context()?;
        }
        1 => {
            p.context()?;
            p.funded(3)?;
        }
        2 => {
            for (offset, local) in [(8, 4), (0, 5), (4, 6)] {
                p.need(&[T(1, 1), T(7, offset), T(2, local)])?;
            }
            p.zero(3)?;
            p.context()?;
            p.funded(4)?;
        }
        _ => return Err(invariant()),
    }
    let rows = m.program.runtime().element_layouts();
    m.take(rows.len())?;
    for row in rows.iter().filter(|row| row.target() == StorageTarget::Linear32V1) {
        p.need(&[T(1, 0), T(6, i64::from(row.element().index())), T(12, 0), T(21, 0)])?;
        let ceiling = crate::generic_execution_v1::checked_i64(
            zryna_ownership_runtime_abi::MAX_VEC_ELEMENTS
                .min(zryna_ownership_runtime_abi::MAX_DYNAMIC_ALLOCATION_BYTES / row.stride()),
        )?;
        match role {
            0 => {
                p.need(&[
                    T(1, 1),
                    T(6, ceiling),
                    T(32, 0),
                    T(2, 4),
                    T(1, 1),
                    T(14, 0),
                    T(14, 0),
                    T(1, 4),
                    T(19, 0),
                    T(2, 5),
                ])?;
                admit(p, 4, 5)?;
                bound(p, 4)?;
                p.need(&[
                    T(1, 0),
                    T(1, 1),
                    T(1, 2),
                    T(31, i64::from(m.container_base + C::VEC_ALLOCATE)),
                    T(2, 6),
                ])?;
                complete(p, 2, 6, 4)?;
            }
            1 => release(p, row)?,
            2 => {
                storage::valid(p, row, 5, 6, 4)?;
                p.need(&[
                    T(1, 2),
                    T(6, ceiling),
                    T(32, 0),
                    T(2, 7),
                    T(1, 2),
                    T(1, 4),
                    T(16, 0),
                    T(1, 7),
                    T(19, 0),
                    T(2, 8),
                ])?;
                admit(p, 9, 8)?;
                bound(p, 7)?;
                p.need(&[
                    T(1, 0),
                    T(1, 1),
                    T(1, 2),
                    T(1, 3),
                    T(31, i64::from(m.container_base + C::VEC_RESERVE)),
                    T(2, 10),
                ])?;
                complete(p, 3, 10, 9)?;
            }
            _ => return Err(invariant()),
        }
        p.need(&[T(28, 0)])?;
    }
    p.fail(255, 255)
}
fn release(
    p: &mut Proof<'_, '_, '_, '_, '_>,
    row: &zryna_ownership_runtime_abi::generic_v1::ElementLayout,
) -> Result<(), Diagnostic> {
    let m = p.model;
    storage::valid(p, row, 1, 2, 3)?;
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
        T(1, 3),
        T(31, i64::from(m.container_base + C::VEC_RELEASE)),
        T(2, 4),
        T(1, 4),
        T(21, 0),
    ])?;
    p.fail(255, 255)?;
    p.need(&[T(28, 0), T(6, 0), T(22, 0)])?;
    Ok(())
}
fn bound(p: &mut Proof<'_, '_, '_, '_, '_>, possible: i64) -> Result<(), Diagnostic> {
    p.need(&[T(1, possible), T(14, 0), T(21, 0)])?;
    p.fail(3, 2)?;
    p.need(&[T(28, 0)])
}
fn admit(p: &mut Proof<'_, '_, '_, '_, '_>, debt: i64, positive: i64) -> Result<(), Diagnostic> {
    p.need(&[
        T(4, 11),
        T(4, 12),
        T(9, 0),
        T(6, 1),
        T(9, 0),
        T(1, debt),
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
        T(1, debt),
        T(9, 0),
        T(5, 12),
    ])
}
fn complete(
    p: &mut Proof<'_, '_, '_, '_, '_>,
    output: i64,
    status: i64,
    debt: i64,
) -> Result<(), Diagnostic> {
    p.need(&[T(1, status), T(21, 0), T(4, 12), T(1, debt), T(10, 0), T(5, 12)])?;
    p.zero(output)?;
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
        T(14, 0),
        T(21, 0),
    ])?;
    p.fail(255, 255)?;
    p.need(&[T(28, 0), T(1, status), T(22, 0), T(28, 0), T(6, 0), T(22, 0)])
}
