//! Exact fixed-kernel cursor guards/progress/backedges; source CFG uses semantic events.
use super::{A, Diagnostic, FunctionBody, Model, invariant, need};
use zryna_ir::generic_v1::execution_v1::CloneNodeKind;
use zryna_layout::TypeCategory;
pub(super) fn check(
    body: &FunctionBody<'_>,
    index: usize,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    if index == 0 {
        need(body, &[A(30, 0), A(29, 0), A(1, 3), A(1, 2), A(17, 0), A(24, 1)], m)?;
        need(body, &[A(1, 3), A(6, 1), A(9, 0), A(2, 3), A(25, 0), A(28, 0), A(28, 0)], m)?;
    }
    let n = m.types.len();
    if index >= 5 && index < 5 + n {
        let ty = index - 5;
        if let Some(node) = m.clones[ty]
            && let CloneNodeKind::FixedArray { length } = node.kind()
        {
            let child = node.element().ok_or_else(invariant)?.ty();
            need(
                body,
                &[A(30, 0), A(29, 0), A(1, 3), A(6, length.cast_signed()), A(17, 0), A(24, 1)],
                m,
            )?;
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
                ],
                m,
            )?;
            need(
                body,
                &[
                    A(31, i64::from(5 + child) + crate::generic_execution_v1::checked_i64(n)?),
                    A(25, 0),
                    A(28, 0),
                    A(28, 0),
                    A(1, 2),
                    A(22, 0),
                    A(28, 0),
                ],
                m,
            )?;
            need(body, &[A(1, 3), A(6, 1), A(9, 0), A(2, 3), A(25, 0), A(28, 0), A(28, 0)], m)?;
        }
    }
    if index >= 5 + n && index < m.source_base as usize {
        let t = m.types[index - 5 - n];
        if t.category() == TypeCategory::FixedArray {
            let length = t.array_length().ok_or_else(invariant)?;
            let stride = t.array_stride().ok_or_else(invariant)?;
            let child = t.referenced_type().ok_or_else(invariant)?.index();
            need(
                body,
                &[
                    A(6, length.cast_signed()),
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
                    A(6, stride.cast_signed()),
                    A(11, 0),
                    A(1, 0),
                    A(9, 0),
                    A(31, i64::from(5 + child) + crate::generic_execution_v1::checked_i64(n)?),
                    A(25, 0),
                    A(28, 0),
                    A(28, 0),
                ],
                m,
            )?;
        }
    }
    Ok(())
}
