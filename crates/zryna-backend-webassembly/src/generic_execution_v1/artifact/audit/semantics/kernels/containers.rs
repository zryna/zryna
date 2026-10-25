//! Independent private allocation/count guards and source-bound metadata dispatch.
use super::{A, Diagnostic, FunctionBody, Model, invariant, need};
use crate::generic_execution_v1::model::containers as C;
use zryna_layout::StorageTarget;
mod loops;
pub(super) fn check(
    body: &FunctionBody<'_>,
    index: usize,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    loops::check(body, index, m)?;
    if index < m.container_base as usize {
        return Ok(());
    }
    let role = crate::generic_execution_v1::checked_u32(index)? - m.container_base;
    match role {
        C::ALLOCATE_RAW => allocate(body, m)?,
        C::RELEASE_RAW => {
            valid(body, m)?;
            for (offset, arg) in [(4, 1), (12, 2)] {
                need(
                    body,
                    &[A(1, 0), A(6, 16), A(10, 0), A(7, offset), A(1, arg), A(13, 0), A(21, 0)],
                    m,
                )?;
            }
        }
        C::VEC_ALLOCATE | C::VEC_RELEASE => vector(body, m, role == C::VEC_ALLOCATE)?,
        C::STRONG_CLONE
        | C::DOWNGRADE
        | C::WEAK_CLONE
        | C::STRONG_BEGIN
        | C::STRONG_FINISH
        | C::WEAK_RELEASE => counts(body, m, role)?,
        _ => return Err(invariant()),
    }
    Ok(())
}
fn valid(body: &FunctionBody<'_>, m: &Model<'_, '_, '_>) -> Result<(), Diagnostic> {
    need(
        body,
        &[
            A(1, 0),
            A(6, i64::from(m.memory.claims().heap_start) + 16),
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
    )?;
    need(body, &[A(1, 0), A(6, 16), A(10, 0), A(7, 8), A(6, 1), A(13, 0), A(21, 0)], m)
}
fn allocate(body: &FunctionBody<'_>, m: &Model<'_, '_, '_>) -> Result<(), Diagnostic> {
    need(
        body,
        &[
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
        ],
        m,
    )?;
    need(body, &[A(4, 5), A(6, 1), A(9, 0), A(5, 5)], m)?;
    need(
        body,
        &[
            A(1, 0),
            A(
                6,
                crate::generic_execution_v1::checked_i64(
                    zryna_ownership_runtime_abi::MAX_DYNAMIC_ALLOCATION_BYTES,
                )?,
            ),
            A(16, 0),
            A(21, 0),
            A(6, 3),
            A(5, 1),
        ],
        m,
    )?;
    need(
        body,
        &[
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
            A(6, 16 * 1024 * 1024),
            A(16, 0),
            A(18, 0),
            A(21, 0),
        ],
        m,
    )?;
    need(body, &[A(1, 4), A(5, 0)], m)?;
    for global in [7, 8] {
        need(body, &[A(4, global), A(6, 1), A(9, 0), A(5, global)], m)?;
    }
    if m.observed() {
        need(
            body,
            &[A(4, 3), A(6, 2), A(12, 0), A(4, 5), A(4, 4), A(12, 0), A(19, 0), A(21, 0)],
            m,
        )?;
    }
    Ok(())
}
fn vector(
    body: &FunctionBody<'_>,
    m: &Model<'_, '_, '_>,
    allocate: bool,
) -> Result<(), Diagnostic> {
    let rows = m.program.runtime().element_layouts();
    m.take(rows.len())?;
    let mut has_linear_elements = false;
    for row in rows.iter().filter(|v| v.target() == StorageTarget::Linear32V1) {
        has_linear_elements = true;
        need(body, &[A(1, 0), A(6, i64::from(row.element().index())), A(12, 0), A(21, 0)], m)?;
        need(
            body,
            &[
                A(1, if allocate { 1 } else { 3 }),
                A(
                    6,
                    crate::generic_execution_v1::checked_i64(
                        (zryna_ownership_runtime_abi::MAX_DYNAMIC_ALLOCATION_BYTES) / row.stride(),
                    )?,
                ),
                A(16, 0),
                A(21, 0),
                A(6, 3),
                A(5, 1),
            ],
            m,
        )?;
    }
    if allocate && has_linear_elements {
        let scratch = i64::from(m.memory.claims().scratch_start);
        need(
            body,
            &[A(6, scratch + 4), A(7, 0), A(21, 0), A(6, scratch + 4), A(7, 0), A(22, 0), A(28, 0)],
            m,
        )?;
    } else if !allocate {
        need(body, &[A(1, 2), A(1, 3), A(16, 0), A(21, 0)], m)?;
    }
    Ok(())
}
fn counts(body: &FunctionBody<'_>, m: &Model<'_, '_, '_>, role: u32) -> Result<(), Diagnostic> {
    valid(body, m)?;
    need(body, &[A(1, 0), A(6, 16), A(10, 0), A(7, 4), A(6, 8), A(15, 0)], m)?;
    if role == C::STRONG_FINISH {
        need(body, &[A(7, 12), A(6, 256), A(19, 0), A(14, 0), A(18, 0), A(21, 0)], m)?;
    } else {
        need(body, &[A(7, 12), A(6, 256), A(19, 0), A(18, 0), A(21, 0)], m)?;
    }
    match role {
        C::STRONG_CLONE | C::DOWNGRADE | C::WEAK_CLONE => {
            let field = if role == C::STRONG_CLONE { 0 } else { 4 };
            let live = if role == C::WEAK_CLONE { 4 } else { 0 };
            need(body, &[A(1, 0), A(7, live), A(14, 0), A(21, 0)], m)?;
            need(body, &[A(1, 0), A(7, field), A(6, -1), A(12, 0), A(21, 0), A(6, 4), A(5, 1)], m)?;
        }
        C::STRONG_BEGIN => {
            need(
                body,
                &[A(8, 0), A(1, 0), A(7, 0), A(14, 0), A(21, 0), A(1, 0), A(6, 16), A(10, 0)],
                m,
            )?;
            need(body, &[A(7, 12), A(6, 256), A(18, 0), A(8, 12)], m)?;
        }
        C::STRONG_FINISH => {
            need(body, &[A(1, 0), A(7, 0), A(1, 0), A(7, 4), A(14, 0), A(18, 0), A(21, 0)], m)?;
            need(body, &[A(7, 12), A(6, 255), A(19, 0), A(8, 12)], m)?;
            need(
                body,
                &[
                    A(1, 0),
                    A(7, 4),
                    A(14, 0),
                    A(21, 0),
                    A(1, 0),
                    A(6, 16),
                    A(10, 0),
                    A(6, 0),
                    A(8, 8),
                ],
                m,
            )?;
        }
        C::WEAK_RELEASE => {
            need(
                body,
                &[A(1, 0), A(7, 4), A(1, 0), A(7, 0), A(14, 0), A(14, 0), A(32, 0), A(21, 0)],
                m,
            )?;
            need(
                body,
                &[
                    A(1, 0),
                    A(7, 0),
                    A(1, 0),
                    A(7, 4),
                    A(18, 0),
                    A(14, 0),
                    A(21, 0),
                    A(1, 0),
                    A(6, 16),
                    A(10, 0),
                    A(6, 0),
                    A(8, 8),
                ],
                m,
            )?;
        }
        _ => return Err(invariant()),
    }
    Ok(())
}
pub(super) fn writes(
    expected: &mut [u32; 10],
    role: u32,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    let rows = m.program.runtime().element_layouts();
    m.take(rows.len())?;
    let n = crate::generic_execution_v1::checked_u32(
        rows.iter().filter(|v| v.target() == StorageTarget::Linear32V1).count(),
    )?;
    expected[1] = match role {
        C::ALLOCATE_RAW => 3 + u32::from(m.observed()),
        C::RELEASE_RAW | C::STRONG_CLONE | C::DOWNGRADE | C::WEAK_CLONE => 5,
        C::VEC_ALLOCATE => n + 1,
        C::VEC_RELEASE => n + 2,
        C::STRONG_BEGIN | C::STRONG_FINISH | C::WEAK_RELEASE => 4,
        _ => return Err(invariant()),
    };
    if role == C::ALLOCATE_RAW {
        for slot in [0, 5, 7, 8] {
            expected[slot] = 1;
        }
    }
    if matches!(role, C::RELEASE_RAW | C::STRONG_FINISH | C::WEAK_RELEASE) {
        expected[8] = 1;
    }
    Ok(())
}
