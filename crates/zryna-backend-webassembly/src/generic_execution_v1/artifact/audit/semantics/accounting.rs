//! Complete actual decoded String-accounting obligations, independent of emitted body recipes.
use super::{Diagnostic, Model, invariant};
use wasmparser::{BlockType, FunctionBody, Operator as O, OperatorsReader, ValType};
mod adapters;
mod controls;
mod vectors;
mod wrappers;
#[derive(Clone, Copy, PartialEq, Eq)]
struct T(u8, i64);
struct Proof<'r, 'm, 'p, 'a, 's> {
    reader: OperatorsReader<'r>,
    model: &'m Model<'p, 'a, 's>,
}
impl Proof<'_, '_, '_, '_, '_> {
    fn need(&mut self, expected: &[T]) -> Result<(), Diagnostic> {
        // Fixed stack obligations: no retained prefix/vector allocation exists on this path.
        self.model.take(expected.len())?;
        for token in expected {
            if self.reader.eof() || atom(&self.reader.read().map_err(|_| invariant())?) != *token {
                return Err(invariant());
            }
        }
        Ok(())
    }
    fn fail(&mut self, source: i64, runtime: i64) -> Result<(), Diagnostic> {
        self.need(&[T(6, source), T(5, 1), T(6, runtime), T(22, 0)])
    }
    fn zero(&mut self, out: i64) -> Result<(), Diagnostic> {
        for offset in [0, 4, 8] {
            self.need(&[T(1, out), T(6, 0), T(8, offset)])?;
        }
        Ok(())
    }
    fn context(&mut self) -> Result<(), Diagnostic> {
        self.need(&[
            T(4, 10),
            T(6, 1_048_576),
            T(16, 0),
            T(4, 8),
            T(4, 10),
            T(16, 0),
            T(18, 0),
            T(4, 8),
            T(6, 1_048_576),
            T(16, 0),
            T(18, 0),
            T(4, 11),
            T(6, 4_194_304),
            T(16, 0),
            T(18, 0),
            T(4, 12),
            T(6, 4_194_304),
            T(16, 0),
            T(18, 0),
            T(21, 0),
        ])?;
        self.fail(255, 255)?;
        self.need(&[T(28, 0), T(4, 11), T(4, 12), T(9, 0), T(6, 4_194_304), T(16, 0), T(21, 0)])?;
        self.fail(255, 255)?;
        self.need(&[T(28, 0)])
    }
    fn funded(&mut self, length: i64) -> Result<(), Diagnostic> {
        self.need(&[
            T(4, 12),
            T(14, 0),
            T(1, length),
            T(14, 0),
            T(14, 0),
            T(4, 8),
            T(14, 0),
            T(19, 0),
            T(18, 0),
            T(21, 0),
        ])?;
        self.fail(255, 255)?;
        self.need(&[T(28, 0)])
    }
}
pub(super) fn check(
    body: &FunctionBody<'_>,
    index: usize,
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    let wrapper = index >= m.wrapper_base as usize && index < m.observer as usize;
    let role = index.checked_sub(m.accounting_base() as usize);
    let control = m.has_accounted_control() && index >= m.control_base() as usize;
    let control_role = index.checked_sub(m.control_base() as usize);
    let vector = m.has_accounted_vec()
        && index >= m.vector_base() as usize
        && index < m.control_base() as usize;
    let vector_role = index.checked_sub(m.vector_base() as usize);
    let locals = if control {
        match control_role {
            Some(0) => 6,
            Some(1..=3 | 7) => 4,
            Some(4..=6) => 3,
            _ => return Err(invariant()),
        }
    } else if vector {
        match vector_role {
            Some(0) => 4,
            Some(1) => 1,
            Some(2) if m.continuation => 7,
            _ => return Err(invariant()),
        }
    } else if wrapper {
        1
    } else {
        match role {
            Some(0 | 1) => 4,
            Some(2) => 1,
            Some(3) if m.concat => 5,
            _ => return Err(invariant()),
        }
    };
    m.take(3)?;
    let mut local = body.get_locals_reader().map_err(|_| invariant())?;
    if local.get_count() != 1 || local.read().map_err(|_| invariant())? != (locals, ValType::I32) {
        return Err(invariant());
    }
    let mut proof =
        Proof { reader: body.get_operators_reader().map_err(|_| invariant())?, model: m };
    if control {
        controls::check(&mut proof, control_role.ok_or_else(invariant)?)?;
    } else if vector {
        vectors::check(&mut proof, vector_role.ok_or_else(invariant)?)?;
    } else if wrapper {
        wrappers::check(&mut proof, index)?;
    } else {
        adapters::check(&mut proof, role.ok_or_else(invariant)?)?;
    }
    proof.need(&[T(28, 0)])?;
    if !proof.reader.eof() {
        return Err(invariant());
    }
    Ok(())
}
fn atom(op: &O<'_>) -> T {
    match *op {
        O::LocalGet { local_index } => T(1, i64::from(local_index)),
        O::LocalSet { local_index } => T(2, i64::from(local_index)),
        O::LocalTee { local_index } => T(3, i64::from(local_index)),
        O::GlobalGet { global_index } => T(4, i64::from(global_index)),
        O::GlobalSet { global_index } => T(5, i64::from(global_index)),
        O::I32Const { value } => T(6, i64::from(value)),
        O::I32Load { memarg } if memarg.memory == 0 && memarg.align == 2 => {
            T(7, memarg.offset.cast_signed())
        }
        O::I32Store { memarg } if memarg.memory == 0 && memarg.align == 2 => {
            T(8, memarg.offset.cast_signed())
        }
        O::I32Load8U { memarg }
            if memarg.memory == 0 && memarg.align == 0 && memarg.offset == 0 =>
        {
            T(37, 0)
        }
        O::I32Store8 { memarg }
            if memarg.memory == 0 && memarg.align == 0 && memarg.offset == 0 =>
        {
            T(38, 0)
        }
        O::I64Const { value } => T(39, value),
        O::I64Store { memarg } if memarg.memory == 0 && memarg.align == 3 => {
            T(40, memarg.offset.cast_signed())
        }
        O::I64ExtendI32U => T(41, 0),
        O::I64Shl => T(42, 0),
        O::I32Add => T(9, 0),
        O::I32Mul => T(11, 0),
        O::I32LtU => T(15, 0),
        O::I32Sub => T(10, 0),
        O::I32Eq => T(12, 0),
        O::I32Ne => T(13, 0),
        O::I32Eqz => T(14, 0),
        O::I32GtU => T(16, 0),
        O::I32GeU => T(17, 0),
        O::I32Or => T(18, 0),
        O::I32And => T(19, 0),
        O::I32LeU => T(32, 0),
        O::Else => T(36, 0),
        O::If { blockty: BlockType::Empty } => T(21, 0),
        O::Return => T(22, 0),
        O::Unreachable => T(23, 0),
        O::End => T(28, 0),
        O::Call { function_index } => T(31, i64::from(function_index)),
        _ => T(255, 0),
    }
}
#[cfg(test)]
mod tests;
