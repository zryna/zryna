//! Exact generic String declaration wrappers and private bounded buffer allocation.
use super::{BYTE, Body, Diagnostic, FunctionCode, I, Model, WORD, canonical_failure, trace};
use crate::generic_execution_v1::{invariant, model};
use wasm_encoder::{BlockType, MemArg};
mod allocation;
pub(super) fn append(m: &Model<'_, '_, '_>, out: &mut Vec<FunctionCode>) -> Result<(), Diagnostic> {
    out.push(copy(m)?);
    out.push(from(m)?);
    out.push(clone_string(m)?);
    out.push(release(m)?);
    out.push(allocation::allocate(m)?);
    Ok(())
}
fn copy(m: &Model<'_, '_, '_>) -> Result<FunctionCode, Diagnostic> {
    let mut b = Body::new(m, 1)?;
    b.ops([
        I::Block(BlockType::Empty),
        I::Loop(BlockType::Empty),
        I::LocalGet(3),
        I::LocalGet(2),
        I::I32GeU,
        I::BrIf(1),
        I::LocalGet(1),
        I::LocalGet(3),
        I::I32Add,
        I::LocalGet(0),
        I::LocalGet(3),
        I::I32Add,
        I::I32Load8U(BYTE),
        I::I32Store8(BYTE),
        I::LocalGet(3),
        I::I32Const(1),
        I::I32Add,
        I::LocalSet(3),
        I::Br(0),
        I::End,
        I::End,
    ])?;
    FunctionCode::new(3, 0, 1, b)
}
fn from(m: &Model<'_, '_, '_>) -> Result<FunctionCode, Diagnostic> {
    let mut b = Body::new(m, 0)?;
    b.ops([
        I::LocalGet(0),
        I::LocalGet(1),
        I::I32Const(0),
        I::I32Const(1),
        I::LocalGet(2),
        I::Call(model::ALLOCATE),
    ])?;
    FunctionCode::new(3, 1, 0, b)
}
fn valid(m: &Model<'_, '_, '_>, b: &mut Body<'_>) -> Result<(), Diagnostic> {
    b.ops([
        I::LocalGet(0),
        I::I32Const(
            crate::generic_execution_v1::checked_u32(m.memory.claims().heap_start)?.cast_signed()
                + 16,
        ),
        I::I32LtU,
        I::LocalGet(0),
        I::GlobalGet(0),
        I::I32GeU,
        I::I32Or,
        I::If(BlockType::Empty),
    ])?;
    canonical_failure(b, 255, 255)?;
    b.op(&I::End)?;
    b.ops([
        I::LocalGet(0),
        I::I32Const(7),
        I::I32And,
        I::LocalGet(0),
        I::LocalGet(1),
        I::I32Add,
        I::LocalGet(0),
        I::I32LtU,
        I::I32Or,
        I::LocalGet(0),
        I::LocalGet(1),
        I::I32Add,
        I::GlobalGet(0),
        I::I32GtU,
        I::I32Or,
        I::If(BlockType::Empty),
    ])?;
    canonical_failure(b, 255, 255)?;
    b.op(&I::End)?;
    b.ops([
        I::LocalGet(0),
        I::I32Const(16),
        I::I32Sub,
        I::I32Load(MemArg { offset: 8, ..WORD }),
        I::I32Const(1),
        I::I32Ne,
        I::LocalGet(0),
        I::I32Const(16),
        I::I32Sub,
        I::I32Load(MemArg { offset: 4, ..WORD }),
        I::LocalGet(1),
        I::I32Ne,
        I::I32Or,
        I::LocalGet(1),
        I::LocalGet(2),
        I::I32Ne,
        I::I32Or,
        I::If(BlockType::Empty),
    ])?;
    canonical_failure(b, 255, 255)?;
    b.op(&I::End)
}
fn clone_string(m: &Model<'_, '_, '_>) -> Result<FunctionCode, Diagnostic> {
    let mut b = Body::new(m, 0)?;
    empty(&mut b, Some(3))?;
    valid(m, &mut b)?;
    b.ops([I::GlobalGet(6), I::I32Const(1), I::I32Add, I::GlobalSet(6)])?;
    if m.observed() {
        fault(
            m,
            &mut b,
            6,
            6,
            5,
            [vec![I::LocalGet(0)], vec![I::LocalGet(1)], vec![I::LocalGet(0)]],
        )?;
    }
    b.ops([
        I::LocalGet(0),
        I::LocalGet(1),
        I::LocalGet(0),
        I::I32Const(2),
        I::LocalGet(3),
        I::Call(model::ALLOCATE),
    ])?;
    FunctionCode::new(4, 1, 0, b)
}
fn release(m: &Model<'_, '_, '_>) -> Result<FunctionCode, Diagnostic> {
    let mut b = Body::new(m, 0)?;
    empty(&mut b, None)?;
    valid(m, &mut b)?;
    trace(
        m,
        &mut b,
        [
            vec![I::I32Const(3)],
            vec![I::LocalGet(0), I::I32Const(16), I::I32Sub, I::I32Load(WORD)],
            vec![I::LocalGet(0)],
            vec![I::I32Const(0)],
            vec![I::LocalGet(1)],
            vec![I::LocalGet(0)],
        ],
    )?;
    b.ops([
        I::LocalGet(0),
        I::I32Const(16),
        I::I32Sub,
        I::I32Const(0),
        I::I32Store(MemArg { offset: 8, ..WORD }),
        I::GlobalGet(8),
        I::I32Const(1),
        I::I32Sub,
        I::GlobalSet(8),
        I::I32Const(0),
    ])?;
    FunctionCode::new(3, 1, 0, b)
}
fn fault(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    kind: i32,
    counter: u32,
    tag: i32,
    payload: [Vec<I<'static>>; 3],
) -> Result<(), Diagnostic> {
    let [source, length, bytes] = payload;
    b.ops([
        I::GlobalGet(3),
        I::I32Const(kind),
        I::I32Eq,
        I::GlobalGet(counter),
        I::GlobalGet(4),
        I::I32Eq,
        I::I32And,
        I::If(BlockType::Empty),
    ])?;
    trace(
        m,
        b,
        [
            vec![I::I32Const(tag)],
            vec![I::GlobalGet(counter)],
            vec![I::I32Const(0)],
            source,
            length,
            bytes,
        ],
    )?;
    canonical_failure(b, 2, 1)?;
    b.op(&I::End)
}

fn empty(b: &mut Body<'_>, out: Option<u32>) -> Result<(), Diagnostic> {
    b.ops([
        I::LocalGet(1),
        I::I32Eqz,
        I::If(BlockType::Empty),
        I::LocalGet(0),
        I::LocalGet(2),
        I::I32Or,
        I::If(BlockType::Empty),
    ])?;
    canonical_failure(b, 255, 255)?;
    b.op(&I::End)?;
    if let Some(out) = out {
        for offset in [0, 4, 8] {
            b.ops([I::LocalGet(out), I::I32Const(0), I::I32Store(MemArg { offset, ..WORD })])?;
        }
    }
    b.ops([I::I32Const(0), I::Return, I::End])
}
