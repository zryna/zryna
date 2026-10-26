//! Typed semantic mutations of actual decoded instructions, never a replacement positive proof.
use super::{C, Model};
use wasm_encoder::{Encode, Instruction as I};
use wasmparser::{BinaryReader, FunctionBody, Operator as O};
fn base(m: &Model<'_, '_, '_>) -> u32 {
    m.vector_base() + if m.has_accounted_vec() { 2 + u32::from(m.continuation) } else { 0 }
}
fn shared(m: &Model<'_, '_, '_>) -> u32 {
    m.types
        .iter()
        .find(|ty| ty.category() == zryna_layout::TypeCategory::Shared)
        .expect("actual Shared type")
        .id()
        .index()
}
pub(super) fn target(m: &Model<'_, '_, '_>, kind: u8) -> usize {
    let index = match kind {
        0 | 9 | 10 | 13 => base(m),
        1 => base(m) + 1,
        2 | 12 => base(m) + 7,
        7 => base(m) + 5,
        8 => base(m) + 4,
        14 => base(m) + 6,
        5 => m.clone_index(shared(m)),
        6 => m.drop_index(shared(m)),
        3 | 11 => {
            m.source_base
                + u32::try_from(
                    m.program
                        .operations()
                        .find(|op| op.shared_construct().is_some())
                        .expect("source Shared construction")
                        .site()
                        .function,
                )
                .expect("bounded source")
        }
        4 => {
            m.source_base
                + u32::try_from(
                    m.program
                        .operations()
                        .find(|op| op.weak_downgrade().is_some())
                        .expect("source downgrade")
                        .site()
                        .function,
                )
                .expect("bounded source")
        }
        _ => panic!("fixed hostile"),
    };
    usize::try_from(index).expect("actual function index")
}
pub(super) fn change(body: &[u8], kind: u8, m: &Model<'_, '_, '_>) -> Vec<u8> {
    let function = FunctionBody::new(BinaryReader::new(body, 0));
    let mut reader = function.get_operators_reader().expect("actual operators");
    let mut ops = Vec::new();
    while !reader.eof() {
        let start = usize::try_from(reader.original_position()).expect("operator start");
        let op = reader.read().expect("actual operator");
        let end = usize::try_from(reader.original_position()).expect("operator end");
        ops.push((start, end, op));
    }
    let raw = m.container_base
        + match kind {
            12 => C::WEAK_UPGRADE,
            14 => C::WEAK_RELEASE,
            _ => C::ALLOCATE_RAW,
        };
    let call = ops
        .iter()
        .position(|(_, _, op)| matches!(op,O::Call{function_index}if *function_index==raw));
    let hold = ops
        .iter()
        .position(|(_, _, op)| matches!(op, O::GlobalSet { global_index: 12 }))
        .unwrap_or(ops.len());
    let last_and = ops[..hold].iter().rposition(|(_, _, op)| matches!(op, O::I32And));
    for (index, (start, end, op)) in ops.iter().enumerate() {
        let replacement = match (kind, op) {
            (0, O::I32Const { value: 2 })
                if index + 1 < ops.len() && matches!(ops[index + 1].2, O::I32Add | O::I32Mul) =>
            {
                Some(vec![I::I32Const(1)])
            }
            (1, O::I32Const { value: -1 }) => Some(vec![I::I32Const(-2)]),
            (2, O::I32And) if last_and == Some(index) => Some(vec![I::I32Or]),
            (3, O::Call { function_index }) if *function_index == base(m) => {
                Some(vec![I::Call(m.container_base + C::ALLOCATE_RAW)])
            }
            (4, O::Call { function_index }) if *function_index == m.container(C::DOWNGRADE) => {
                Some(vec![I::Call(m.container_base + C::DOWNGRADE)])
            }
            (5, O::Call { function_index }) if *function_index == m.container(C::STRONG_CLONE) => {
                Some(vec![I::Call(m.container_base + C::STRONG_CLONE)])
            }
            (6, O::Call { function_index }) if *function_index == m.container(C::STRONG_BEGIN) => {
                Some(vec![I::Call(m.container_base + C::STRONG_BEGIN)])
            }
            (7, O::I32Load { memarg })
                if memarg.offset == 4
                    && index > 0
                    && matches!(ops[index - 1].2, O::LocalGet { local_index: 0 }) =>
            {
                Some(vec![I::I32Load(wasm_encoder::MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                })])
            }
            (8, O::GlobalSet { global_index: 12 }) => Some(vec![I::GlobalSet(11)]),
            (9, O::I32Sub) if call.is_some_and(|call| index > call) => Some(vec![I::I32Add]),
            (10, O::Call { function_index }) if *function_index == raw => {
                Some(vec![I::Call(raw), I::I32Const(0), I::GlobalSet(10)])
            }
            (11, O::I64Store { memarg }) => Some(vec![
                I::I32WrapI64,
                I::I32Store(wasm_encoder::MemArg {
                    offset: memarg.offset,
                    align: 2,
                    memory_index: 0,
                }),
            ]),
            (12, O::I32Const { value: 5 }) if call.is_some_and(|call| index > call) => {
                Some(vec![I::I32Const(2)])
            }
            (13, O::I32Const { value: 4_194_304 }) => Some(vec![I::I32Const(4_194_305)]),
            (14, O::I32Load { .. }) if call.is_some_and(|call| index > call) => {
                Some(vec![I::Drop, I::I32Const(0)])
            }
            _ => None,
        };
        if let Some(replacement) = replacement {
            let mut out = body[..*start].to_vec();
            for op in replacement {
                op.encode(&mut out);
            }
            out.extend_from_slice(&body[*end..]);
            return out;
        }
    }
    panic!("hostile {kind} must change an actual owned instruction")
}
