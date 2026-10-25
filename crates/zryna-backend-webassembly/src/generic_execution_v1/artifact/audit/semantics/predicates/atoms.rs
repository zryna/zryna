//! Fixed atoms preserve the actual typed loads and empty control signatures.
use wasmparser::{BlockType, Operator as O};
#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) struct A(pub(super) u8, pub(super) i64);
pub(super) fn decode(op: &O<'_>) -> A {
    match *op {
        O::LocalGet { local_index } => A(1, i64::from(local_index)),
        O::LocalTee { local_index } => A(3, i64::from(local_index)),
        O::GlobalGet { global_index } => A(4, i64::from(global_index)),
        O::I32Const { value } => A(6, i64::from(value)),
        O::I32Load { memarg } if memarg.memory == 0 && memarg.align == 2 => {
            i64::try_from(memarg.offset).map_or(A(255, 0), |offset| A(7, offset))
        }
        O::I32Add => A(9, 0),
        O::I32Sub => A(10, 0),
        O::I32Eq => A(12, 0),
        O::I32Ne => A(13, 0),
        O::I32Eqz => A(14, 0),
        O::I32LtU => A(15, 0),
        O::I32GtU => A(16, 0),
        O::I32GeU => A(17, 0),
        O::I32Or => A(18, 0),
        O::I32And => A(19, 0),
        O::If { blockty: BlockType::Empty } => A(21, 0),
        O::I32LeU => A(32, 0),
        _ => A(255, 0),
    }
}
