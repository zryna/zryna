//! Three-record private `StringConcat` adapter; allocation never reads a combined range from one input.
use super::{Body, Diagnostic, FunctionCode, I, Model, WORD, canonical_failure, model, trace};
use wasm_encoder::{BlockType, MemArg};
mod records;
pub(super) fn append(m: &Model<'_, '_, '_>, out: &mut Vec<FunctionCode>) -> Result<(), Diagnostic> {
    if m.concat {
        out.push(super::containers::allocation::allocate(m)?);
        out.push(concat(m)?);
    }
    Ok(())
}
fn concat(m: &Model<'_, '_, '_>) -> Result<FunctionCode, Diagnostic> {
    let mut b = Body::new(m, 8)?;
    for (record, start) in [(0, 3), (1, 6)] {
        for (offset, local) in [(0, start), (4, start + 1), (8, start + 2)] {
            b.load(record, offset)?;
            b.op(&I::LocalSet(local))?;
        }
    }
    for offset in [0, 4, 8] {
        b.ops([I::LocalGet(2), I::I32Const(0), I::I32Store(MemArg { offset, ..WORD })])?;
    }
    records::valid(m, &mut b, 3)?;
    records::valid(m, &mut b, 6)?;
    b.ops([
        I::LocalGet(4),
        I::LocalGet(7),
        I::I32Add,
        I::LocalSet(9),
        I::LocalGet(9),
        I::LocalGet(4),
        I::I32LtU,
        I::LocalGet(9),
        I::I32Const(
            crate::generic_execution_v1::checked_u32(
                zryna_ownership_runtime_abi::MAX_DYNAMIC_ALLOCATION_BYTES,
            )?
            .cast_signed(),
        ),
        I::I32GtU,
        I::I32Or,
        I::If(BlockType::Empty),
    ])?;
    canonical_failure(&mut b, 3, 2)?;
    b.ops([
        I::End,
        I::LocalGet(9),
        I::I32Eqz,
        I::If(BlockType::Empty),
        I::I32Const(0),
        I::Return,
        I::End,
    ])?;
    let scratch =
        crate::generic_execution_v1::checked_u32(m.memory.claims().scratch_start)?.cast_signed();
    b.ops([
        I::I32Const(scratch),
        I::LocalGet(9),
        I::I32Const(1),
        I::Call(m.concat_allocator()),
        I::I64Store(MemArg { offset: 0, align: 3, memory_index: 0 }),
        I::I32Const(scratch + 4),
        I::I32Load(WORD),
        I::If(BlockType::Empty),
        I::I32Const(scratch + 4),
        I::I32Load(WORD),
        I::Return,
        I::End,
        I::I32Const(scratch),
        I::I32Load(WORD),
        I::LocalSet(10),
        I::LocalGet(3),
        I::LocalGet(10),
        I::LocalGet(4),
        I::Call(model::COPY),
        I::LocalGet(6),
        I::LocalGet(10),
        I::LocalGet(4),
        I::I32Add,
        I::LocalGet(7),
        I::Call(model::COPY),
    ])?;
    for (offset, local) in [(0, 10), (4, 9), (8, 9)] {
        b.ops([I::LocalGet(2), I::LocalGet(local), I::I32Store(MemArg { offset, ..WORD })])?;
    }
    if m.observed() {
        m.take(6 * (size_of::<Vec<I<'static>>>() + size_of::<I<'static>>()))?;
        trace(
            m,
            &mut b,
            [
                vec![I::I32Const(1)],
                vec![I::GlobalGet(7)],
                vec![I::LocalGet(10)],
                vec![I::I32Const(0)],
                vec![I::LocalGet(9)],
                vec![I::LocalGet(10)],
            ],
        )?;
    }
    b.op(&I::I32Const(0))?;
    FunctionCode::new(3, 1, 8, b)
}
