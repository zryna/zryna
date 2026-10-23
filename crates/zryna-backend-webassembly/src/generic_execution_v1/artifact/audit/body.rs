//! Exact recipe bytes plus independent operator/local/call/memory capability checks.
use super::{Diagnostic, FunctionCode, Model, budget, invariant, sections};
use wasmparser::{BlockType, FunctionBody, Operator, ValType};
pub(super) fn check(
    bytes: &[u8],
    actual: &FunctionBody<'_>,
    index: usize,
    m: &Model<'_, '_, '_>,
    expected: &FunctionCode,
) -> Result<(), Diagnostic> {
    super::semantics::check(actual, index, m)?;
    let range = actual.range();
    let range = usize::try_from(range.start).map_err(|_| budget())?
        ..usize::try_from(range.end).map_err(|_| budget())?;
    let (parameters, results) = sections::signature(m, index)?;
    if expected.result64 != sections::wide(m, index)
        || expected.parameters != parameters
        || expected.results != results
        || bytes.get(range) != Some(expected.body.bytes.as_slice())
    {
        return Err(invariant());
    }
    let mut locals = actual.get_locals_reader().map_err(|_| invariant())?;
    if expected.locals == 0 {
        if locals.get_count() != 0 {
            return Err(invariant());
        }
    } else if locals.get_count() != 1
        || locals.read().map_err(|_| invariant())? != (expected.locals, ValType::I32)
    {
        return Err(invariant());
    }
    let maximum = parameters.checked_add(expected.locals).ok_or_else(budget)?;
    let mut ops = actual.get_operators_reader().map_err(|_| invariant())?;
    while !ops.eof() {
        operation(&ops.read().map_err(|_| invariant())?, maximum, m)?;
    }
    Ok(())
}
fn operation(op: &Operator<'_>, locals: u32, m: &Model<'_, '_, '_>) -> Result<(), Diagnostic> {
    match op {
        Operator::LocalGet { local_index }
        | Operator::LocalSet { local_index }
        | Operator::LocalTee { local_index }
            if *local_index < locals => {}
        Operator::GlobalGet { global_index } | Operator::GlobalSet { global_index }
            if *global_index < 10 => {}
        Operator::Call { function_index } if *function_index < m.total_functions() => {}
        Operator::I32Load { memarg } | Operator::I32Store { memarg }
            if memarg.memory == 0 && memarg.align == 2 && memarg.offset <= 16 => {}
        Operator::I64Store { memarg }
            if memarg.memory == 0 && memarg.align == 3 && memarg.offset == 0 => {}
        Operator::I32Load8U { memarg } | Operator::I32Store8 { memarg }
            if memarg.memory == 0 && memarg.align == 0 && memarg.offset == 0 => {}
        Operator::Block { blockty: BlockType::Empty }
        | Operator::Loop { blockty: BlockType::Empty }
        | Operator::If { blockty: BlockType::Empty }
        | Operator::Br { .. }
        | Operator::BrIf { .. }
        | Operator::Else
        | Operator::End
        | Operator::Return
        | Operator::Unreachable
        | Operator::I32Const { .. }
        | Operator::I32Add
        | Operator::I32Sub
        | Operator::I32Mul
        | Operator::I32Eq
        | Operator::I32Ne
        | Operator::I32LtS
        | Operator::I32LeS
        | Operator::I32GtS
        | Operator::I32GeS
        | Operator::I32LtU
        | Operator::I32GtU
        | Operator::I32GeU
        | Operator::I32Eqz
        | Operator::I32Or
        | Operator::I32And
        | Operator::I32LeU
        | Operator::I32ShrU
        | Operator::I64Const { .. }
        | Operator::I64ExtendI32U
        | Operator::I64Shl
        | Operator::I64Or => {}
        _ => return Err(invariant()),
    }
    Ok(())
}
