//! Private checked buffer header/heap commit; no carrier allocations.
use super::{
    BlockType, Body, Diagnostic, FunctionCode, I, MemArg, Model, WORD, canonical_failure, fault,
    invariant, model, trace,
};
pub(super) fn allocate(m: &Model<'_, '_, '_>) -> Result<FunctionCode, Diagnostic> {
    let mut b = Body::new(m, 4)?;
    b.ops([I::LocalGet(1), I::I32Eqz, I::If(BlockType::Empty)])?;
    for offset in [0, 4, 8] {
        b.ops([I::LocalGet(4), I::I32Const(0), I::I32Store(MemArg { offset, ..WORD })])?;
    }
    b.ops([I::I32Const(0), I::Return, I::End])?;
    b.ops([I::GlobalGet(5), I::I32Const(1), I::I32Add, I::GlobalSet(5)])?;
    if m.observed() {
        fault(
            m,
            &mut b,
            2,
            5,
            4,
            [vec![I::LocalGet(2)], vec![I::LocalGet(1)], vec![I::LocalGet(0)]],
        )?;
    }
    b.ops([
        I::LocalGet(1),
        I::I32Const(
            i32::try_from(zryna_ownership_runtime_abi::MAX_DYNAMIC_ALLOCATION_BYTES - 16)
                .map_err(|_| invariant())?,
        ),
        I::I32GtU,
        I::If(BlockType::Empty),
    ])?;
    canonical_failure(&mut b, 3, 2)?;
    b.op(&I::End)?;
    b.ops([
        I::GlobalGet(0),
        I::LocalTee(5),
        I::I32Const(16),
        I::I32Add,
        I::LocalTee(6),
        I::LocalGet(1),
        I::I32Add,
        I::I32Const(7),
        I::I32Add,
        I::I32Const(-8),
        I::I32And,
        I::LocalTee(7),
        I::LocalGet(5),
        I::I32LtU,
        I::LocalGet(7),
        I::I32Const(
            crate::generic_execution_v1::checked_u32(
                crate::generic_execution_v1::memory::MEMORY_BYTES,
            )?
            .cast_signed(),
        ),
        I::I32GtU,
        I::I32Or,
        I::If(BlockType::Empty),
    ])?;
    trace(
        m,
        &mut b,
        [
            vec![I::I32Const(4)],
            vec![I::GlobalGet(5)],
            vec![I::I32Const(0)],
            vec![I::LocalGet(2)],
            vec![I::LocalGet(1)],
            vec![I::LocalGet(0)],
        ],
    )?;
    canonical_failure(&mut b, 2, 1)?;
    b.op(&I::End)?;
    commit(m, &mut b)?;
    FunctionCode::new(5, 1, 4, b)
}
fn commit(m: &Model<'_, '_, '_>, b: &mut Body<'_>) -> Result<(), Diagnostic> {
    b.ops([
        I::LocalGet(7),
        I::GlobalSet(0),
        I::GlobalGet(7),
        I::I32Const(1),
        I::I32Add,
        I::LocalTee(8),
        I::GlobalSet(7),
        I::GlobalGet(8),
        I::I32Const(1),
        I::I32Add,
        I::GlobalSet(8),
    ])?;
    for (offset, value) in [
        (0, vec![I::LocalGet(8)]),
        (4, vec![I::LocalGet(1)]),
        (8, vec![I::I32Const(1)]),
        (12, vec![I::I32Const(0)]),
    ] {
        b.op(&I::LocalGet(5))?;
        b.ops(value)?;
        b.op(&I::I32Store(MemArg { offset, ..WORD }))?;
    }
    b.ops([I::LocalGet(0), I::LocalGet(6), I::LocalGet(1), I::Call(model::COPY)])?;
    for (offset, local) in [(0, 6), (4, 1), (8, 1)] {
        b.ops([I::LocalGet(4), I::LocalGet(local), I::I32Store(MemArg { offset, ..WORD })])?;
    }
    trace(
        m,
        b,
        [
            vec![I::LocalGet(3)],
            vec![I::LocalGet(8)],
            vec![I::LocalGet(6)],
            vec![I::LocalGet(2)],
            vec![I::LocalGet(1)],
            vec![I::LocalGet(6)],
        ],
    )?;
    b.op(&I::I32Const(0))?;
    Ok(())
}
