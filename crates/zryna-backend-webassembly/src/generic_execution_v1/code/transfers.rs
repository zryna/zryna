//! Exact selected successors and cleanup of the owning Return/Trap only.
use super::{Body, Diagnostic, I, Model, WORD, model, source as s};
use crate::generic_execution_v1::invariant;
mod borrowed;
mod upgrade;
use wasm_encoder::BlockType;
use zryna_ir::generic_v1::execution_v1::{VerifiedBlock, raw};
pub(super) fn terminator(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    f: usize,
    block: VerifiedBlock<'_, '_>,
) -> Result<(), Diagnostic> {
    match block.terminator() {
        raw::Terminator::Return { value, .. } => {
            if let Some(id) = value {
                let slot = s::value(m, f, *id)?;
                b.addr(0, slot.offset)?;
                b.ops([
                    I::LocalGet(1),
                    I::I32Const(
                        crate::generic_execution_v1::checked_u32(slot.bytes)?.cast_signed(),
                    ),
                    I::Call(model::COPY),
                ])?;
            }
            s::cleanup(m, b, block.exit_cleanup().ok_or_else(invariant)?)?;
            b.ops([I::I32Const(0), I::Return])?;
        }
        raw::Terminator::Trap { identity, .. } => {
            s::cleanup(m, b, block.exit_cleanup().ok_or_else(invariant)?)?;
            let status = match identity {
                raw::TrapIdentity::Bounds => 1,
                raw::TrapIdentity::Allocation => 2,
                raw::TrapIdentity::Capacity => 3,
                raw::TrapIdentity::Refcount => 4,
                raw::TrapIdentity::Utf8 => 5,
            };
            b.ops([I::I32Const(status), I::GlobalSet(1), I::I32Const(status), I::Return])?;
        }
        raw::Terminator::Jump(e) => edge(m, b, f, e, 0, 1)?,
        raw::Terminator::Branch { condition, yes, no } => {
            s::scalar(b, s::value(m, f, *condition)?)?;
            b.op(&I::If(BlockType::Empty))?;
            edge(m, b, f, yes, 0, 2)?;
            b.op(&I::End)?;
            edge(m, b, f, no, 0, 1)?;
        }
        raw::Terminator::ClosedEnumMatch { mode, .. } if *mode != raw::MatchMode::Value => {
            borrowed::emit(m, b, f, block)?;
        }
        raw::Terminator::ClosedEnumMatch { ty, scrutinee, arms, .. } => {
            let src = s::referent(m, f, *scrutinee)?;
            for arm in arms {
                b.addr(0, src.offset)?;
                b.ops([
                    I::I32Load(WORD),
                    I::I32Const(
                        crate::generic_execution_v1::checked_u32(arm.ordinal)?.cast_signed(),
                    ),
                    I::I32Eq,
                    I::If(BlockType::Empty),
                ])?;
                let skip = if let Some(binding) = &arm.binding {
                    let raw::MatchBinding::Value(value) = binding else { return Err(invariant()) };
                    let dst = s::value(m, f, value.id)?;
                    b.addr(
                        0,
                        src.offset
                            + crate::generic_execution_v1::checked_u32(
                                m.types[*ty as usize].payload().ok_or_else(invariant)?.0,
                            )?,
                    )?;
                    b.addr(0, dst.offset)?;
                    b.ops([
                        I::I32Const(
                            crate::generic_execution_v1::checked_u32(dst.bytes)?.cast_signed(),
                        ),
                        I::Call(model::COPY),
                    ])?;
                    1
                } else {
                    0
                };
                edge(m, b, f, &arm.edge, skip, 2)?;
                b.op(&I::End)?;
            }
            b.op(&I::Unreachable)?;
        }
        raw::Terminator::WeakUpgradeBranch { .. } => upgrade::emit(m, b, f, block)?,
    }
    Ok(())
}
fn edge(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    f: usize,
    edge: &raw::Edge,
    skip: usize,
    depth: u32,
) -> Result<(), Diagnostic> {
    m.take(edge.target as usize + 1)?;
    let target = m.functions[f].blocks().nth(edge.target as usize).ok_or_else(invariant)?;
    for (argument, parameter) in edge.arguments.iter().zip(target.value_parameters().skip(skip)) {
        s::copy(b, s::value(m, f, *argument)?, s::value(m, f, parameter.id())?)?;
    }
    b.ops([
        I::I32Const(crate::generic_execution_v1::checked_u32(edge.target)?.cast_signed()),
        I::LocalSet(2),
        I::Br(depth),
    ])
}
