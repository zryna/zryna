//! Scalar wrapper reset and exit authority, derived from checked parameter/result storage.
use super::{Diagnostic, Proof, T, invariant};
use zryna_ir::generic_v1::execution_v1::{VerifiedParameter, raw};
pub(super) fn check(p: &mut Proof<'_, '_, '_, '_, '_>, index: usize) -> Result<(), Diagnostic> {
    let m = p.model;
    let selected = m.program.export_functions()[index - m.wrapper_base as usize];
    let function = m.functions[selected];
    let count = crate::generic_execution_v1::checked_i64(function.parameters().len())?;
    p.need(&[
        T(4, 12),
        T(21, 0),
        T(23, 0),
        T(28, 0),
        T(4, 2),
        T(4, 8),
        T(18, 0),
        T(21, 0),
        T(23, 0),
        T(28, 0),
        T(6, 1),
        T(5, 2),
    ])?;
    for global in [10, 11, 12, 1, 5, 6, 7, 9] {
        p.need(&[T(6, 0), T(5, global)])?;
    }
    p.need(&[T(6, i64::from(m.memory.claims().heap_start)), T(5, 0)])?;
    m.take(function.parameters().len())?;
    for (ordinal, parameter) in function.parameters().enumerate() {
        let VerifiedParameter::Value(value) = parameter else { return Err(invariant()) };
        let slot = &m.memory.claims().functions[selected].values[value.id() as usize];
        let ordinal = crate::generic_execution_v1::checked_i64(ordinal)?;
        if slot.bytes == 1 {
            p.need(&[
                T(1, ordinal),
                T(6, 1),
                T(16, 0),
                T(21, 0),
                T(6, 0),
                T(5, 2),
                T(23, 0),
                T(28, 0),
            ])?;
        }
        p.need(&[
            T(
                6,
                crate::generic_execution_v1::checked_i64(
                    m.memory.claims().frames_start + slot.offset,
                )?,
            ),
            T(1, ordinal),
            T(
                if slot.bytes == 1 {
                    38
                } else if slot.bytes == 4 {
                    8
                } else {
                    return Err(invariant());
                },
                0,
            ),
        ])?;
    }
    p.need(&[
        T(6, i64::from(m.memory.claims().frames_start)),
        T(6, crate::generic_execution_v1::checked_i64(m.memory.claims().scratch_start + 16)?),
        T(31, i64::from(m.source_base) + crate::generic_execution_v1::checked_i64(selected)?),
        T(2, count),
        T(6, 0),
        T(5, 2),
        T(4, 8),
        T(4, 12),
        T(18, 0),
        T(21, 0),
        T(23, 0),
        T(28, 0),
        T(1, count),
        T(21, 0),
        T(23, 0),
        T(28, 0),
        T(4, 8),
        T(21, 0),
        T(23, 0),
        T(28, 0),
    ])?;
    let raw::ValueType::Stored(ty) = function.result_type() else { return Err(invariant()) };
    p.need(&[
        T(6, crate::generic_execution_v1::checked_i64(m.memory.claims().scratch_start + 16)?),
        T(
            if m.types[ty as usize].size() == 1 {
                37
            } else if m.types[ty as usize].size() == 4 {
                7
            } else {
                return Err(invariant());
            },
            0,
        ),
    ])
}
