//! Independent actual-operator verification against fixed material and source-bound relocations.
use super::{CLONE, DROP, Diagnostic, Model, SCHEMA, call, constant, invariant, template};
use wasmparser::{FunctionBody, Operator, ValType};

pub(in crate::generic_execution_v1) fn check_body(
    actual: &FunctionBody<'_>,
    index: usize,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    let local = m.recursive_function(index).ok_or_else(invariant)?;
    let mut work = m.work.get();
    let material = template::load(&mut work)?;
    m.work.set(work);
    let expected = material.get(local).ok_or_else(invariant)?;
    locals(actual, SCHEMA[local].3)?;
    m.take(actual.as_bytes().len())?;
    let mut a = actual.get_operators_reader().map_err(|_| invariant())?;
    let mut b = expected.get_operators_reader().map_err(|_| invariant())?;
    while !b.eof() {
        if a.eof() {
            return Err(invariant());
        }
        let astart = a.original_position();
        let bstart = b.original_position();
        let ao = a.read().map_err(|_| invariant())?;
        let bo = b.read().map_err(|_| invariant())?;
        match bo {
            Operator::Call { function_index } => {
                if !matches!(ao,Operator::Call{function_index:i}if i==call(m,function_index)?) {
                    return Err(invariant());
                }
            }
            Operator::GlobalGet { global_index: 3 } => {
                if !matches!(ao,Operator::GlobalGet{global_index:i}if i==m.recursive_used_global())
                {
                    return Err(invariant());
                }
            }
            Operator::GlobalGet { global_index } => {
                if !matches!(ao,Operator::I32Const{value}if value==constant(m,global_index)?) {
                    return Err(invariant());
                }
            }
            Operator::GlobalSet { global_index: 3 } => {
                if !matches!(ao,Operator::GlobalSet{global_index:i}if i==m.recursive_used_global())
                {
                    return Err(invariant());
                }
            }
            _ => {
                let ar = actual.range();
                let av = actual
                    .as_bytes()
                    .get(
                        usize::try_from(astart.checked_sub(ar.start).ok_or_else(invariant)?)
                            .map_err(|_| invariant())?
                            ..usize::try_from(
                                a.original_position()
                                    .checked_sub(ar.start)
                                    .ok_or_else(invariant)?,
                            )
                            .map_err(|_| invariant())?,
                    )
                    .ok_or_else(invariant)?;
                let bv = template::BINARY
                    .get(
                        usize::try_from(bstart).map_err(|_| invariant())?
                            ..usize::try_from(b.original_position()).map_err(|_| invariant())?,
                    )
                    .ok_or_else(invariant)?;
                if av != bv {
                    return Err(invariant());
                }
            }
        }
    }
    if !a.eof() {
        return Err(invariant());
    }
    Ok(())
}
fn locals(body: &FunctionBody<'_>, count: u32) -> Result<(), Diagnostic> {
    let mut reader = body.get_locals_reader().map_err(|_| invariant())?;
    if count == 0 {
        if reader.get_count() != 0 {
            return Err(invariant());
        }
    } else if reader.get_count() != 1
        || reader.read().map_err(|_| invariant())? != (count, ValType::I32)
    {
        return Err(invariant());
    }
    Ok(())
}
pub(in crate::generic_execution_v1) fn check_wrapper(
    body: &FunctionBody<'_>,
    index: usize,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    locals(body, 0)?;
    let n = m.types.len();
    let mut ops = body.get_operators_reader().map_err(|_| invariant())?;
    if index < 5 + n {
        let id = index.checked_sub(5).ok_or_else(invariant)?;
        if m.clones.get(id).ok_or_else(invariant)?.is_none() {
            if !matches!(ops.read(), Ok(Operator::Unreachable)) {
                return Err(invariant());
            }
        } else {
            need(&mut ops, u32::try_from(id).map_err(|_| invariant())?)?;
            local(&mut ops, 0)?;
            local(&mut ops, 1)?;
            target(&mut ops, m.recursive_base() + CLONE)?;
        }
    } else {
        let id = index.checked_sub(5 + n).ok_or_else(invariant)?;
        let ty = m.types.get(id).ok_or_else(invariant)?;
        if ty.drop_kind() != 0 {
            need(&mut ops, u32::try_from(id).map_err(|_| invariant())?)?;
            local(&mut ops, 0)?;
            need(&mut ops, u32::try_from(ty.size()).map_err(|_| invariant())?)?;
            target(&mut ops, m.recursive_base() + DROP)?;
        }
    }
    if !matches!(ops.read(), Ok(Operator::End)) || !ops.eof() {
        return Err(invariant());
    }
    Ok(())
}
fn need(ops: &mut wasmparser::OperatorsReader<'_>, value: u32) -> Result<(), Diagnostic> {
    if !matches!(ops.read(),Ok(Operator::I32Const{value:v})if v==value.cast_signed()) {
        return Err(invariant());
    }
    Ok(())
}
fn local(ops: &mut wasmparser::OperatorsReader<'_>, index: u32) -> Result<(), Diagnostic> {
    if !matches!(ops.read(),Ok(Operator::LocalGet{local_index:i})if i==index) {
        return Err(invariant());
    }
    Ok(())
}
fn target(ops: &mut wasmparser::OperatorsReader<'_>, index: u32) -> Result<(), Diagnostic> {
    if !matches!(ops.read(),Ok(Operator::Call{function_index:i})if i==index) {
        return Err(invariant());
    }
    Ok(())
}
