//! Relocate fixed material into the already verified module; no imports survive this adapter.
use super::{CLONE, DROP, Diagnostic, Model, SCHEMA, budget, call, constant, invariant, template};
use crate::generic_execution_v1::code::{Body, FunctionCode};
use wasm_encoder::{BlockType as B, Instruction as I, MemArg, ValType};
use wasmparser::{BlockType, Operator};

pub(in crate::generic_execution_v1) fn append(
    m: &Model<'_, '_, '_>,
    out: &mut Vec<FunctionCode>,
) -> Result<(), Diagnostic> {
    if m.recursive_count() == 0 {
        return Ok(());
    }
    let mut work = m.work.get();
    let bodies = template::load(&mut work)?;
    m.work.set(work);
    for (index, body) in bodies.iter().enumerate() {
        let (p, r, wide, locals) = SCHEMA[index];
        let mut b = Body::new(m, locals)?;
        let mut reader = body.get_operators_reader().map_err(|_| invariant())?;
        while !reader.eof() {
            let op = reader.read().map_err(|_| invariant())?;
            if reader.eof() && matches!(op, Operator::End) {
                break;
            }
            b.op(&instruction(&op, m)?)?;
        }
        out.push(FunctionCode {
            parameters: p,
            results: r,
            locals,
            result64: wide,
            body: b.finish()?,
        });
    }
    Ok(())
}
pub(in crate::generic_execution_v1) fn clone_wrapper(
    m: &Model<'_, '_, '_>,
    ty: u32,
) -> Result<FunctionCode, Diagnostic> {
    let mut b = Body::new(m, 0)?;
    if m.clones[ty as usize].is_none() {
        b.op(&I::Unreachable)?;
    } else {
        b.ops([
            I::I32Const(ty.cast_signed()),
            I::LocalGet(0),
            I::LocalGet(1),
            I::Call(m.recursive_base() + CLONE),
        ])?;
    }
    Ok(FunctionCode { parameters: 2, results: 1, locals: 0, result64: false, body: b.finish()? })
}
pub(in crate::generic_execution_v1) fn drop_wrapper(
    m: &Model<'_, '_, '_>,
    ty: u32,
) -> Result<FunctionCode, Diagnostic> {
    let mut b = Body::new(m, 0)?;
    if m.types[ty as usize].drop_kind() != 0 {
        let size = u32::try_from(m.types[ty as usize].size()).map_err(|_| budget())?;
        b.ops([
            I::I32Const(ty.cast_signed()),
            I::LocalGet(0),
            I::I32Const(size.cast_signed()),
            I::Call(m.recursive_base() + DROP),
        ])?;
    }
    Ok(FunctionCode { parameters: 1, results: 0, locals: 0, result64: false, body: b.finish()? })
}
fn block(kind: BlockType) -> Result<B, Diagnostic> {
    match kind {
        BlockType::Empty => Ok(B::Empty),
        BlockType::Type(wasmparser::ValType::I32) => Ok(B::Result(ValType::I32)),
        BlockType::Type(wasmparser::ValType::I64) => Ok(B::Result(ValType::I64)),
        _ => Err(invariant()),
    }
}
fn memory(arg: wasmparser::MemArg) -> Result<MemArg, Diagnostic> {
    if arg.memory != 0 {
        return Err(invariant());
    }
    Ok(MemArg { offset: arg.offset, align: arg.align.into(), memory_index: 0 })
}
fn instruction(op: &Operator<'_>, m: &Model<'_, '_, '_>) -> Result<I<'static>, Diagnostic> {
    Ok(match op {
        Operator::Call { function_index } => I::Call(call(m, *function_index)?),
        Operator::GlobalGet { global_index: 3 } => I::GlobalGet(m.recursive_used_global()),
        Operator::GlobalGet { global_index } => I::I32Const(constant(m, *global_index)?),
        Operator::GlobalSet { global_index: 3 } => I::GlobalSet(m.recursive_used_global()),
        Operator::LocalGet { local_index } => I::LocalGet(*local_index),
        Operator::LocalSet { local_index } => I::LocalSet(*local_index),
        Operator::LocalTee { local_index } => I::LocalTee(*local_index),
        Operator::I32Const { value } => I::I32Const(*value),
        Operator::I64Const { value } => I::I64Const(*value),
        Operator::I32Load { memarg } => I::I32Load(memory(*memarg)?),
        Operator::I32Store { memarg } => I::I32Store(memory(*memarg)?),
        Operator::I32Load8U { memarg } => I::I32Load8U(memory(*memarg)?),
        Operator::I32Store8 { memarg } => I::I32Store8(memory(*memarg)?),
        Operator::I64Store { memarg } => I::I64Store(memory(*memarg)?),
        Operator::Block { blockty } => I::Block(block(*blockty)?),
        Operator::Loop { blockty } => I::Loop(block(*blockty)?),
        Operator::If { blockty } => I::If(block(*blockty)?),
        Operator::Br { relative_depth } => I::Br(*relative_depth),
        Operator::BrIf { relative_depth } => I::BrIf(*relative_depth),
        Operator::Else => I::Else,
        Operator::End => I::End,
        Operator::Return => I::Return,
        Operator::Unreachable => I::Unreachable,
        Operator::Drop => I::Drop,
        Operator::Select => I::Select,
        Operator::I32Add => I::I32Add,
        Operator::I32Sub => I::I32Sub,
        Operator::I32Mul => I::I32Mul,
        Operator::I32DivU => I::I32DivU,
        Operator::I32Eq => I::I32Eq,
        Operator::I32Ne => I::I32Ne,
        Operator::I32LtU => I::I32LtU,
        Operator::I32LeU => I::I32LeU,
        Operator::I32GtU => I::I32GtU,
        Operator::I32GeU => I::I32GeU,
        Operator::I32Eqz => I::I32Eqz,
        Operator::I32Or => I::I32Or,
        Operator::I32And => I::I32And,
        Operator::I32Shl => I::I32Shl,
        Operator::I32ShrU => I::I32ShrU,
        Operator::I64ExtendI32U => I::I64ExtendI32U,
        Operator::I64Shl => I::I64Shl,
        Operator::I64Or => I::I64Or,
        _ => return Err(invariant()),
    })
}
