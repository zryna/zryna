//! Independent guarded kernel/observation policy and global-write ownership.
use super::{Diagnostic, Model, budget, invariant, sections};
use wasmparser::{FunctionBody, Operator as O};
mod containers;
mod locals;
mod loops;
mod observation;
mod patterns;
use patterns::{A, need};
pub(super) fn check(
    body: &FunctionBody<'_>,
    index: usize,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    writes(body, index, m)?;
    containers::check(body, index, m)?;
    locals::check(body, index, m)?;
    loops::check(body, index, m)?;
    match index {
        0 => need(body, &[A(1, 3), A(1, 2), A(17, 0), A(24, 1)], m)?,
        2 | 3 => {
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
                    A(21, 0),
                ],
                m,
            )?;
            need(body, &[A(1, 0), A(6, 16), A(10, 0), A(7, 8), A(6, 1), A(13, 0)], m)?;
            need(body, &[A(1, 0), A(1, 1), A(9, 0), A(4, 0), A(16, 0), A(18, 0), A(21, 0)], m)?;
            need(body, &[A(1, 1), A(1, 2), A(13, 0), A(18, 0), A(21, 0)], m)?;
            if index == 2 {
                need(body, &[A(4, 6), A(6, 1), A(9, 0), A(5, 6)], m)?;
                if m.observed() {
                    fault(body, 6, 6, m)?;
                }
            } else {
                need(body, &[A(4, 8), A(6, 1), A(10, 0), A(5, 8)], m)?;
            }
        }
        4 => allocator(body, m)?,
        _ if index >= m.wrapper_base as usize && index < m.observer as usize => {
            wrapper(body, index, m)?;
        }
        _ if m.observed() && index == m.observer as usize => {
            need(body, &[A(4, 8), A(4, 2), A(18, 0), A(21, 0), A(23, 0)], m)?;
            need(body, &[A(1, 0), A(6, 0x00ff_ffff), A(19, 0), A(5, 4)], m)?;
            need(body, &[A(1, 1), A(6, 4096), A(16, 0), A(18, 0), A(21, 0), A(23, 0)], m)?;
        }
        _ if m.observed() && index == m.recorder as usize => observer(body, m)?,
        _ => {}
    }
    observation::check(body, index, m)?;
    Ok(())
}
fn fault(
    body: &FunctionBody<'_>,
    kind: i64,
    counter: i64,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    need(
        body,
        &[A(4, 3), A(6, kind), A(12, 0), A(4, counter), A(4, 4), A(12, 0), A(19, 0), A(21, 0)],
        m,
    )
}
fn wrapper(body: &FunctionBody<'_>, index: usize, m: &Model<'_, '_, '_>) -> Result<(), Diagnostic> {
    need(body, &[A(4, 2), A(4, 8), A(18, 0), A(21, 0), A(23, 0)], m)?;
    need(body, &[A(6, i64::from(m.memory.claims().heap_start)), A(5, 0)], m)?;
    need(body, &[A(4, 8), A(21, 0), A(23, 0)], m)?;
    let selected = m.program.export_functions()[index - m.wrapper_base as usize];
    let count = crate::generic_execution_v1::checked_i64(m.functions[selected].parameters().len())?;
    need(
        body,
        &[
            A(31, i64::from(m.source_base) + crate::generic_execution_v1::checked_i64(selected)?),
            A(2, count),
            A(6, 0),
            A(5, 2),
            A(1, count),
            A(21, 0),
            A(23, 0),
            A(28, 0),
        ],
        m,
    )?;
    need(body, &[A(23, 0), A(28, 0), A(6, 1), A(5, 2)], m)?;
    for global in [1, 5, 6, 7, 9] {
        need(body, &[A(6, 0), A(5, global)], m)?;
    }
    Ok(())
}
fn observer(body: &FunctionBody<'_>, m: &Model<'_, '_, '_>) -> Result<(), Diagnostic> {
    need(body, &[A(6, 4097), A(5, 9), A(22, 0)], m)?;
    need(
        body,
        &[
            A(6, i64::from(m.memory.claims().trace_start)),
            A(4, 9),
            A(6, 4),
            A(11, 0),
            A(9, 0),
            A(2, 6),
        ],
        m,
    )?;
    need(body, &[A(1, 4), A(6, 4091), A(16, 0)], m)?;
    need(
        body,
        &[A(4, 9), A(1, 4), A(6, 5), A(9, 0), A(9, 0), A(6, 4096), A(16, 0), A(18, 0), A(21, 0)],
        m,
    )
}
fn writes(body: &FunctionBody<'_>, index: usize, m: &Model<'_, '_, '_>) -> Result<(), Diagnostic> {
    let mut expected = [0u32; 10];
    match index {
        2 => {
            expected[1] = 4 + u32::from(m.observed());
            expected[6] = 1;
        }
        3 => {
            expected[1] = 4;
            expected[8] = 1;
        }
        4 => {
            expected[0] = 1;
            expected[1] = 2 + u32::from(m.observed());
            expected[5] = 1;
            expected[7] = 1;
            expected[8] = 1;
        }
        _ => {}
    }
    if index >= m.source_base as usize && index < m.wrapper_base as usize {
        let f = m.functions[index - m.source_base as usize];
        m.take(f.blocks().len())?;
        for block in f.blocks() {
            expected[1] += u32::from(matches!(
                block.terminator(),
                zryna_ir::generic_v1::execution_v1::raw::Terminator::Trap { .. }
            ));
            m.take(block.operations().len())?;
            for op in block.operations() {
                expected[1] += u32::from(matches!(
                    op.kind(),
                    zryna_ir::generic_v1::execution_v1::raw::Operation::IndexCopy { .. }
                ));
            }
        }
    }
    if index >= m.wrapper_base as usize && index < m.observer as usize {
        let f = m.functions[m.program.export_functions()[index - m.wrapper_base as usize]];
        for global in [0, 1, 5, 6, 7, 9] {
            expected[global] = 1;
        }
        expected[2] = 2;
        m.take(f.parameters().len())?;
        for param in f.parameters() {
            if let zryna_ir::generic_v1::execution_v1::VerifiedParameter::Value(v) = param
                && let zryna_ir::generic_v1::execution_v1::raw::ValueType::Stored(ty) = v.ty()
            {
                expected[2] += u32::from(m.types[ty as usize].size() == 1);
            }
        }
    }
    if m.observed() && index == m.observer as usize {
        expected[3] = 1;
        expected[4] = 1;
    }
    if m.observed() && index == m.recorder as usize {
        expected[9] = 2;
    }
    if index >= m.container_base as usize {
        containers::writes(
            &mut expected,
            crate::generic_execution_v1::checked_u32(index)? - m.container_base,
            m,
        )?;
    }
    let mut actual = [0u32; 10];
    let mut ops = body.get_operators_reader().map_err(|_| invariant())?;
    while !ops.eof() {
        m.take(1)?;
        if let O::GlobalSet { global_index } = ops.read().map_err(|_| invariant())? {
            let count = actual.get_mut(global_index as usize).ok_or_else(invariant)?;
            *count = count.checked_add(1).ok_or_else(budget)?;
        }
    }
    if actual != expected {
        return Err(invariant());
    }
    Ok(())
}

fn allocator(body: &FunctionBody<'_>, m: &Model<'_, '_, '_>) -> Result<(), Diagnostic> {
    need(body, &[A(4, 5), A(6, 1), A(9, 0), A(5, 5)], m)?;
    need(
        body,
        &[
            A(1, 1),
            A(
                6,
                crate::generic_execution_v1::checked_i64(
                    zryna_ownership_runtime_abi::MAX_DYNAMIC_ALLOCATION_BYTES - 16,
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
            A(3, 5),
            A(6, 16),
            A(9, 0),
            A(3, 6),
            A(1, 1),
            A(9, 0),
            A(6, 7),
            A(9, 0),
            A(6, -8),
            A(19, 0),
            A(3, 7),
        ],
        m,
    )?;
    need(
        body,
        &[
            A(3, 7),
            A(1, 5),
            A(15, 0),
            A(1, 7),
            A(6, 16 * 1024 * 1024),
            A(16, 0),
            A(18, 0),
            A(21, 0),
        ],
        m,
    )?;
    need(body, &[A(1, 7), A(5, 0)], m)?;
    need(body, &[A(4, 8), A(6, 1), A(9, 0), A(5, 8)], m)?;
    if m.observed() {
        fault(body, 2, 5, m)?;
    }
    Ok(())
}
