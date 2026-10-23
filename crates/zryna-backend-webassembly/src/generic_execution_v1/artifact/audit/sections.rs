//! Independent typed section and actual static partition checks.
use super::{Diagnostic, Model, budget, invariant};
use wasmparser::{
    DataKind, DataSectionReader, ExportSectionReader, ExternalKind, FunctionSectionReader,
    GlobalSectionReader, MemorySectionReader, Operator, TypeSectionReader, ValType,
};
pub(super) fn types(
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
pub(super) fn signature(m: &Model<'_, '_, '_>, index: usize) -> Result<(u32, u32), Diagnostic> {
    let index = crate::generic_execution_v1::checked_u32(index)?;
    if index >= m.container_base && index < m.total_functions() {
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
pub(super) fn functions(reader: FunctionSectionReader<'_>, count: usize) -> Result<(), Diagnostic> {
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
pub(super) fn memory(reader: MemorySectionReader<'_>) -> Result<(), Diagnostic> {
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
pub(super) fn globals(
    reader: GlobalSectionReader<'_>,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    if reader.count() != 10 {
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
pub(super) fn exports(
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
pub(super) fn data(reader: DataSectionReader<'_>, m: &Model<'_, '_, '_>) -> Result<(), Diagnostic> {
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
            || data.data.len() != m.memory.claims().heap_start as usize
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
        if data.data[cursor..].iter().any(|byte| *byte != 0) {
            return Err(invariant());
        }
    }
    Ok(())
}

pub(super) fn wide(m: &Model<'_, '_, '_>, index: usize) -> bool {
    m.container_count != 0 && index == m.container_base as usize
}
fn container_signature(role: u32) -> (u32, u32) {
    [(2, 1), (3, 1), (3, 1), (4, 1), (1, 1), (1, 1), (1, 1), (2, 1), (1, 1), (2, 1)][role as usize]
}
