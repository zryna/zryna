//! Small fixed operator windows for guarded heap and checked test instrumentation.
use super::{Diagnostic, FunctionBody, Model, O, invariant};
#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) struct A(pub(super) u8, pub(super) i64);
pub(super) fn need(
    body: &FunctionBody<'_>,
    expected: &[A],
    m: &Model<'_, '_, '_>,
) -> Result<(), Diagnostic> {
    if expected.is_empty() || expected.len() > 32 {
        return Err(invariant());
    }
    let mut ring = [A(255, 0); 32];
    let mut count = 0usize;
    let mut ops = body.get_operators_reader().map_err(|_| invariant())?;
    while !ops.eof() {
        m.take(1)?;
        ring[count % expected.len()] = atom(&ops.read().map_err(|_| invariant())?);
        count += 1;
        if count >= expected.len()
            && (0..expected.len()).all(|i| ring[(count + i) % expected.len()] == expected[i])
        {
            return Ok(());
        }
    }
    Err(invariant())
}
fn atom(op: &O<'_>) -> A {
    match *op {
        O::LocalGet { local_index } => A(1, i64::from(local_index)),
        O::LocalSet { local_index } => A(2, i64::from(local_index)),
        O::LocalTee { local_index } => A(3, i64::from(local_index)),
        O::GlobalGet { global_index } => A(4, i64::from(global_index)),
        O::GlobalSet { global_index } => A(5, i64::from(global_index)),
        O::I32Const { value } => A(6, i64::from(value)),
        O::I32Load { memarg } => A(7, memarg.offset.cast_signed()),
        O::I32Store { memarg } => A(8, memarg.offset.cast_signed()),
        O::I32Add => A(9, 0),
        O::I32Sub => A(10, 0),
        O::I32Mul => A(11, 0),
        O::I32Eq => A(12, 0),
        O::I32Ne => A(13, 0),
        O::I32Eqz => A(14, 0),
        O::I32LtU => A(15, 0),
        O::I32GtU => A(16, 0),
        O::I32GeU => A(17, 0),
        O::I32Or => A(18, 0),
        O::I32And => A(19, 0),
        O::I32ShrU => A(20, 0),
        O::I32LeU => A(32, 0),
        O::I64Const { value } => A(33, value),
        O::I64ExtendI32U => A(34, 0),
        O::I64Store { .. } => A(35, 0),
        O::If { .. } => A(21, 0),
        O::Return => A(22, 0),
        O::Unreachable => A(23, 0),
        O::BrIf { relative_depth } => A(24, i64::from(relative_depth)),
        O::Br { relative_depth } => A(25, i64::from(relative_depth)),
        O::End => A(28, 0),
        O::Loop { .. } => A(29, 0),
        O::Block { .. } => A(30, 0),
        O::Call { function_index } => A(31, i64::from(function_index)),
        _ => A(255, 0),
    }
}
