//! Independent typed section and actual static partition checks.
use super::{Diagnostic, Model, budget, invariant};
use wasmparser::{
    DataKind, DataSectionReader, ExportSectionReader, ExternalKind, FunctionSectionReader,
    GlobalSectionReader, MemorySectionReader, Operator, TypeSectionReader, ValType,
};
pub(in crate::generic_execution_v1::artifact) fn types(
    reader: TypeSectionReader<'_>,
    m: &Model<'_, '_, '_>,
    count: usize,
) -> Result<(), Diagnostic> {
    if reader.count() as usize != count {
        return Err(invariant());
    }
    for (index, ty) in reader.into_iter_err_on_gc_types().enumerate() {
        let ty = ty.map_err(|_| invariant())?;
        let (parameters, results) = signature(m, index)?;
        if ty.params().len() != parameters as usize
            || ty.results().len() != results as usize
            || ty.params().iter().any(|ty| *ty != ValType::I32)
            || ty
                .results()
                .iter()
                .any(|ty| *ty != if wide(m, index) { ValType::I64 } else { ValType::I32 })
        {
            return Err(invariant());
        }
    }
    Ok(())
}
pub(in crate::generic_execution_v1::artifact) fn signature(
    m: &Model<'_, '_, '_>,
    index: usize,
) -> Result<(u32, u32), Diagnostic> {
    if let Some(local) = m.recursive_function(index) {
        let (p, r, _, _) = crate::generic_execution_v1::recursive::SCHEMA[local];
        return Ok((p, r));
    }
    let index = crate::generic_execution_v1::checked_u32(index)?;
    if m.accounted() && index >= m.accounting_base() {
        if m.has_accounted_control() && index >= m.control_base() {
            return [(2, 1), (1, 1), (1, 1), (1, 1), (2, 1), (1, 1), (2, 1), (1, 1)]
                .get((index - m.control_base()) as usize)
                .copied()
                .ok_or_else(invariant);
        }
        if index >= m.vector_base() && index < m.control_base() && m.has_accounted_vec() {
            return [(3, 1), (4, 1), (4, 1)]
                .get((index - m.vector_base()) as usize)
                .copied()
                .ok_or_else(invariant);
        }
        return [(3, 1), (4, 1), (3, 1), (3, 1)]
            .get((index - m.accounting_base()) as usize)
            .copied()
            .ok_or_else(invariant);
    }
    if m.concat && index == m.concat_allocator() {
        return Ok((2, 1));
    }
    if m.concat && index == m.concat_index() {
        return Ok((3, 1));
    }
    if index >= m.container_base && index < m.container_base + m.container_count {
        return Ok(container_signature(index - m.container_base));
    }
    if index < 5 {
        return Ok([(3, 0), (3, 1), (4, 1), (3, 1), (5, 1)][index as usize]);
    }
    let n = crate::generic_execution_v1::checked_u32(m.types.len())?;
    if index < 5 + n {
        return Ok((2, 1));
    }
    if index < m.source_base {
        return Ok((1, 0));
    }
    if index < m.wrapper_base {
        return Ok((2, 1));
    }
    if index < m.observer {
        let selected = m
            .program
            .export_functions()
            .get((index - m.wrapper_base) as usize)
            .ok_or_else(invariant)?;
        return Ok((
            crate::generic_execution_v1::checked_u32(m.functions[*selected].parameters().len())?,
            1,
        ));
    }
    if m.observed() && index == m.observer {
        return Ok((1, 1));
    }
    if m.observed() && index == m.recorder {
        return Ok((6, 0));
    }
    Err(invariant())
}
pub(in crate::generic_execution_v1::artifact) fn functions(
    reader: FunctionSectionReader<'_>,
    count: usize,
) -> Result<(), Diagnostic> {
    if reader.count() as usize != count {
        return Err(invariant());
    }
    for (index, ty) in reader.into_iter().enumerate() {
        if ty.map_err(|_| invariant())? as usize != index {
            return Err(invariant());
        }
    }
    Ok(())
}
pub(in crate::generic_execution_v1::artifact) fn memory(
    reader: MemorySectionReader<'_>,
) -> Result<(), Diagnostic> {
    if reader.count() != 1 {
        return Err(invariant());
    }
    for memory in reader {
        let memory = memory.map_err(|_| invariant())?;
        if memory.initial != 256
            || memory.maximum != Some(256)
            || memory.memory64
            || memory.shared
            || memory.page_size_log2.is_some()
        {
            return Err(invariant());
        }
    }
    Ok(())
}
pub(in crate::generic_execution_v1::artifact) fn globals(
    reader: GlobalSectionReader<'_>,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    if m.accounted() {
        m.take(m.global_count() as usize)?;
    }
    if reader.count() != m.global_count() {
        return Err(invariant());
    }
    for (index, global) in reader.into_iter().enumerate() {
        let global = global.map_err(|_| invariant())?;
        let mut init = global.init_expr.get_operators_reader();
        let expected = if index == 0 {
            crate::generic_execution_v1::checked_u32(m.memory.claims().heap_start)?.cast_signed()
        } else {
            0
        };
        if global.ty.content_type != ValType::I32
            || !global.ty.mutable
            || global.ty.shared
            || !matches!(init.read(),Ok(Operator::I32Const{value}) if value==expected)
            || !matches!(init.read(), Ok(Operator::End))
            || !init.eof()
        {
            return Err(invariant());
        }
    }
    Ok(())
}
pub(in crate::generic_execution_v1::artifact) fn exports(
    reader: ExportSectionReader<'_>,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    let count = m.program.export_functions().len();
    if reader.count() as usize != count + usize::from(m.observed()) {
        return Err(invariant());
    }
    for (index, export) in reader.into_iter().enumerate() {
        let export = export.map_err(|_| invariant())?;
        let (name, function) = if index < count {
            (
                m.program
                    .scalar_abi()
                    .exports()
                    .nth(index)
                    .ok_or_else(invariant)?
                    .webassembly_name()
                    .as_str(),
                m.wrapper_base + crate::generic_execution_v1::checked_u32(index)?,
            )
        } else {
            ("$zryna$observation", m.observer)
        };
        if export.name != name || export.kind != ExternalKind::Func || export.index != function {
            return Err(invariant());
        }
    }
    Ok(())
}
pub(in crate::generic_execution_v1::artifact) fn data(
    reader: DataSectionReader<'_>,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    if reader.count() != 1 {
        return Err(invariant());
    }
    for data in reader {
        let data = data.map_err(|_| invariant())?;
        let DataKind::Active { memory_index: 0, offset_expr } = data.kind else {
            return Err(invariant());
        };
        let mut offset = offset_expr.get_operators_reader();
        if !matches!(offset.read(), Ok(Operator::I32Const { value: 0 }))
            || !matches!(offset.read(), Ok(Operator::End))
            || !offset.eof()
            || data.data.len() != m.memory.static_data_bytes() as usize
        {
            return Err(invariant());
        }
        let mut cursor = 0usize;
        for literal in &m.memory.claims().literals {
            let start = literal.offset as usize;
            let end = start.checked_add(literal.bytes.len()).ok_or_else(budget)?;
            if data.data.get(cursor..start).ok_or_else(invariant)?.iter().any(|byte| *byte != 0)
                || data.data.get(start..end) != Some(literal.bytes.as_slice())
            {
                return Err(invariant());
            }
            cursor = end;
        }
        if let Some(arena) = m.memory.recursive_arena() {
            let start = arena.metadata_start as usize;
            let end = start.checked_add(arena.metadata_bytes as usize).ok_or_else(budget)?;
            if data.data.get(cursor..start).ok_or_else(invariant)?.iter().any(|b| *b != 0)
                || end != data.data.len()
            {
                return Err(invariant());
            }
            crate::generic_execution_v1::recursive::check_metadata(&data.data[start..end], m)?;
        } else if data.data[cursor..].iter().any(|byte| *byte != 0) {
            return Err(invariant());
        }
    }
    Ok(())
}

pub(in crate::generic_execution_v1::artifact) fn wide(m: &Model<'_, '_, '_>, index: usize) -> bool {
    m.recursive_function(index).is_some_and(|i| crate::generic_execution_v1::recursive::SCHEMA[i].2)
        || (m.container_count != 0 && index == m.container_base as usize)
        || (m.concat && index == m.concat_allocator() as usize)
        || (m.has_accounted_control() && index == m.control_base() as usize)
}
fn container_signature(role: u32) -> (u32, u32) {
    [(2, 1), (3, 1), (3, 1), (4, 1), (1, 1), (1, 1), (1, 1), (2, 1), (1, 1), (2, 1), (4, 1), (1, 1)]
        [role as usize]
}
