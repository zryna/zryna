//! Exact writable local roles; immutable frame/output/formal pointer parameters stay immutable.
use super::{Diagnostic, FunctionBody, Model, O, budget, invariant, sections};
use zryna_ir::generic_v1::execution_v1::{CloneNodeKind, raw};
pub(super) fn check(
    body: &FunctionBody<'_>,
    index: usize,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    let (parameters, _) = sections::signature(m, index)?;
    let declared = if index >= m.container_base as usize {
        if index == m.container_base as usize { 3 } else { 0 }
    } else if index < 5 {
        [1usize, 0, 0, 0, 4][index]
    } else if index < 5 + m.types.len() {
        2
    } else if index < m.source_base as usize {
        1
    } else if index < m.wrapper_base as usize {
        4
    } else if index <= m.observer as usize {
        1
    } else {
        2
    };
    let count = (parameters as usize).checked_add(declared).ok_or_else(budget)?;
    m.take(count.checked_mul(size_of::<u32>() * 2).ok_or_else(budget)?)?;
    let mut expected = Vec::new();
    expected.try_reserve_exact(count).map_err(|_| budget())?;
    expected.resize(count, 0u32);
    match index {
        0 => expected[3] = 1,
        4 => {
            for slot in &mut expected[5..=8] {
                *slot = 1;
            }
        }
        _ => {}
    }
    if index == m.container_base as usize && m.container_count != 0 {
        for slot in &mut expected[2..=4] {
            *slot = 1;
        }
    }
    let types = m.types.len();
    if index >= 5
        && index < 5 + types
        && let Some(node) = m.clones[index - 5]
    {
        expected[2] = match node.kind() {
            CloneNodeKind::Enum => {
                m.take(node.variants().len())?;
                crate::generic_execution_v1::checked_u32(
                    node.variants().filter(|(_, c)| c.is_some()).count(),
                )?
            }
            CloneNodeKind::Struct => crate::generic_execution_v1::checked_u32(node.fields().len())?,
            CloneNodeKind::SharedCount | CloneNodeKind::WeakCount => 1,
            CloneNodeKind::VecEach => {
                expected[3] = 2;
                2
            }
            CloneNodeKind::FixedArray { .. } => {
                expected[3] = 2;
                1
            }
            _ => 0,
        };
    }
    drop_counts(&mut expected, index, m);
    source_counts(&mut expected, index, m)?;
    if index >= m.wrapper_base as usize && index < m.observer as usize {
        let f = m.functions[m.program.export_functions()[index - m.wrapper_base as usize]];
        let slot = expected.get_mut(f.parameters().len()).ok_or_else(invariant)?;
        *slot = 1;
    }
    if m.observed() && index == m.observer as usize {
        expected[1] = 2;
    }
    if m.observed() && index == m.recorder as usize {
        expected[6] = 1;
        expected[7] = 1;
    }
    let mut actual = Vec::new();
    actual.try_reserve_exact(count).map_err(|_| budget())?;
    actual.resize(count, 0u32);
    let mut ops = body.get_operators_reader().map_err(|_| invariant())?;
    while !ops.eof() {
        m.take(1)?;
        match ops.read().map_err(|_| invariant())? {
            O::LocalSet { local_index } | O::LocalTee { local_index } => {
                let n = actual.get_mut(local_index as usize).ok_or_else(invariant)?;
                *n = n.checked_add(1).ok_or_else(budget)?;
            }
            _ => {}
        }
    }
    if actual != expected {
        return Err(invariant());
    }
    Ok(())
}

fn source_counts(
    expected: &mut [u32],
    index: usize,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    if index >= m.source_base as usize && index < m.wrapper_base as usize {
        let f = m.functions[index - m.source_base as usize];
        m.take(f.blocks().len())?;
        for block in f.blocks() {
            expected[2] += match block.terminator() {
                raw::Terminator::Jump(_) => 1,
                raw::Terminator::Branch { .. } => 2,
                raw::Terminator::ClosedEnumMatch { arms, .. } => {
                    crate::generic_execution_v1::checked_u32(arms.len())?
                }
                _ => 0,
            };
            m.take(block.operations().len())?;
            for op in block.operations() {
                match op.kind() {
                    raw::Operation::Clone { .. }
                    | raw::Operation::StringFromUtf8 { .. }
                    | raw::Operation::WeakDowngrade { .. } => {
                        expected[3] += 1;
                    }
                    raw::Operation::SourceCall { .. }
                    | raw::Operation::ClosedGenericCall { .. }
                    | raw::Operation::VecConstruct { .. }
                    | raw::Operation::SharedConstruct { .. } => {
                        expected[3] += 1;
                        expected[4] += 1;
                    }
                    raw::Operation::IndexCopy { .. } => expected[5] += 1,
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

fn drop_counts(expected: &mut [u32], index: usize, m: &Model<'_, '_, '_>) {
    let types = m.types.len();
    if index >= 5 + types
        && index < m.source_base as usize
        && matches!(
            m.types[index - 5 - types].category(),
            zryna_layout::TypeCategory::FixedArray | zryna_layout::TypeCategory::Vec
        )
    {
        expected[1] = 2;
    }
    if index >= 5 + types
        && index < m.source_base as usize
        && m.types[index - 5 - types].category() == zryna_layout::TypeCategory::Shared
    {
        expected[1] = 1;
    }
}
