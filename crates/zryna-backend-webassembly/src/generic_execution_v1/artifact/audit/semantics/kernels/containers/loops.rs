//! Dynamic Vec cursors remain invocation-local and each loop has its exact exit and backedge.
use super::{A, Diagnostic, FunctionBody, Model, invariant, need};
use zryna_ir::generic_v1::execution_v1::CloneNodeKind;
use zryna_layout::TypeCategory;
pub(super) fn check(
    body: &FunctionBody<'_>,
    index: usize,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    let n = m.types.len();
    if index >= 5
        && index < 5 + n
        && m.clones[index - 5].is_some_and(|v| v.kind() == CloneNodeKind::VecEach)
    {
        let ty = crate::generic_execution_v1::checked_u32(index - 5)?;
        let child =
            m.clones[index - 5].ok_or_else(invariant)?.element().ok_or_else(invariant)?.ty();
        let stride = crate::generic_execution_v1::checked_i64(m.element(ty)?.stride())?;
        need(body, &[A(30, 0), A(29, 0), A(1, 3), A(1, 0), A(7, 4), A(17, 0), A(24, 1)], m)?;
        need(
            body,
            &[
                A(30, 0),
                A(29, 0),
                A(1, 3),
                A(14, 0),
                A(24, 1),
                A(1, 3),
                A(6, 1),
                A(10, 0),
                A(2, 3),
                A(1, 1),
                A(7, 0),
                A(1, 3),
                A(6, stride),
                A(11, 0),
                A(9, 0),
                A(31, i64::from(m.drop_index(child))),
                A(25, 0),
                A(28, 0),
                A(28, 0),
            ],
            m,
        )?;
        need(
            body,
            &[
                A(1, 3),
                A(6, 1),
                A(9, 0),
                A(2, 3),
                A(1, 1),
                A(1, 3),
                A(8, 4),
                A(25, 0),
                A(28, 0),
                A(28, 0),
            ],
            m,
        )?;
    }
    if index >= 5 + n
        && index < m.source_base as usize
        && m.types[index - 5 - n].category() == TypeCategory::Vec
    {
        let ty = crate::generic_execution_v1::checked_u32(index - 5 - n)?;
        let stride = crate::generic_execution_v1::checked_i64(m.element(ty)?.stride())?;
        let child = m.element(ty)?.element().index();
        need(
            body,
            &[
                A(1, 0),
                A(7, 4),
                A(2, 1),
                A(30, 0),
                A(29, 0),
                A(1, 1),
                A(14, 0),
                A(24, 1),
                A(1, 1),
                A(6, 1),
                A(10, 0),
                A(3, 1),
                A(6, stride),
                A(11, 0),
                A(1, 0),
                A(7, 0),
                A(9, 0),
                A(31, i64::from(m.drop_index(child))),
                A(25, 0),
                A(28, 0),
                A(28, 0),
            ],
            m,
        )?;
    }
    Ok(())
}
