//! Source-return status is already logical; canonical runtime status never enters this wrapper.
use super::{BYTE, Body, Diagnostic, FunctionCode, I, Model, WORD};
use crate::generic_execution_v1::{budget, checked_u32, invariant};
use wasm_encoder::BlockType;
use zryna_abi::{ScalarType, ScalarValue, VerifiedInvocation};
use zryna_ir::generic_v1::execution_v1::VerifiedParameter;
// Zero params; two fresh I32 locals: logical source status and scalar bits.
pub(in crate::generic_execution_v1) fn build(
    m: &Model<'_, '_, '_>,
    request: &VerifiedInvocation<'_>,
) -> Result<FunctionCode, Diagnostic> {
    let target =
        *m.program.export_functions().get(request.export().index()).ok_or_else(invariant)?;
    let f = m.functions[target];
    let out = m.memory.claims().scratch_start.checked_add(16).ok_or_else(budget)?;
    let heap = checked_u32(m.memory.claims().heap_start)?.cast_signed();
    let mut b = Body::new(m, 2)?;
    for global in [2, 8] {
        guard_global(&mut b, global)?;
    }
    if m.accounted() {
        guard_global(&mut b, 12)?;
    }
    if m.recursive_count() != 0 {
        guard_global(&mut b, m.recursive_used_global())?;
    }
    b.ops([
        I::I32Const(0),
        I::LocalSet(0),
        I::I32Const(0),
        I::LocalSet(1),
        I::I32Const(out.cast_signed()),
        I::I32Const(0),
        I::I32Store(WORD),
        I::I32Const(1),
        I::GlobalSet(2),
    ])?;
    reset(m, &mut b, heap)?;
    for (parameter, value) in f.parameters().zip(request.arguments()) {
        let VerifiedParameter::Value(parameter) = parameter else { return Err(invariant()) };
        let slot = &m.memory.claims().functions[target].values[parameter.id() as usize];
        let address = m.memory.claims().frames_start.checked_add(slot.offset).ok_or_else(budget)?;
        let scalar = match value {
            ScalarValue::Bool(value) => i32::from(*value),
            ScalarValue::I32(value) => *value,
        };
        b.ops([
            I::I32Const(address.cast_signed()),
            I::I32Const(scalar),
            if slot.bytes == 1 {
                I::I32Store8(BYTE)
            } else if slot.bytes == 4 {
                I::I32Store(WORD)
            } else {
                return Err(invariant());
            },
        ])?;
    }
    b.ops([
        I::I32Const(m.memory.claims().frames_start.cast_signed()),
        I::I32Const(out.cast_signed()),
        I::Call(m.source_base + checked_u32(target)?),
        I::LocalSet(0),
        I::LocalGet(0),
        I::GlobalGet(1),
        I::I32Ne,
    ])?;
    guard(&mut b)?;
    b.ops([I::LocalGet(0), I::I32Const(5), I::I32GtU])?;
    guard(&mut b)?;
    // Source and helper cleanup must have completed before scalar publication on either path.
    guard_global(&mut b, 8)?;
    if m.accounted() {
        guard_global(&mut b, 12)?;
        for (global, max) in [(10, 1_048_576), (11, 4_194_304)] {
            b.ops([I::GlobalGet(global), I::I32Const(max), I::I32GtU])?;
            guard(&mut b)?;
        }
    }
    if m.recursive_count() != 0 {
        guard_global(&mut b, m.recursive_used_global())?;
    }
    b.ops([I::GlobalGet(2), I::I32Const(1), I::I32Ne])?;
    guard(&mut b)?;
    publish(m, &mut b, request.export().result(), out, heap)?;
    Ok(FunctionCode { parameters: 0, results: 1, locals: 2, result64: true, body: b.finish()? })
}
fn guard(b: &mut Body<'_>) -> Result<(), Diagnostic> {
    b.ops([I::If(BlockType::Empty), I::Unreachable, I::End])
}
fn guard_global(b: &mut Body<'_>, global: u32) -> Result<(), Diagnostic> {
    b.op(&I::GlobalGet(global))?;
    guard(b)
}
fn reset(m: &Model<'_, '_, '_>, b: &mut Body<'_>, heap: i32) -> Result<(), Diagnostic> {
    for global in [1, 5, 6, 7, 9] {
        b.ops([I::I32Const(0), I::GlobalSet(global)])?;
    }
    if m.accounted() {
        for global in [10, 11, 12] {
            b.ops([I::I32Const(0), I::GlobalSet(global)])?;
        }
    }
    b.ops([I::I32Const(heap), I::GlobalSet(0)])
}

// Scalar publication is one complete post-cleanup phase, before the function's final End.
fn publish(
    m: &Model<'_, '_, '_>,
    b: &mut Body<'_>,
    result: ScalarType,
    out: u32,
    heap: i32,
) -> Result<(), Diagnostic> {
    b.ops([
        I::LocalGet(0),
        I::I32Eqz,
        I::If(BlockType::Empty),
        I::I32Const(out.cast_signed()),
        if result == ScalarType::Bool { I::I32Load8U(BYTE) } else { I::I32Load(WORD) },
        I::LocalSet(1),
    ])?;
    if result == ScalarType::Bool {
        b.ops([I::LocalGet(1), I::I32Const(1), I::I32GtU])?;
        guard(b)?;
    }
    b.op(&I::End)?;
    b.ops([I::I32Const(out.cast_signed()), I::I32Const(0), I::I32Store(WORD)])?;
    reset(m, b, heap)?;
    b.ops([
        I::I32Const(0),
        I::GlobalSet(2),
        I::LocalGet(0),
        I::I64ExtendI32U,
        I::I64Const(32),
        I::I64Shl,
        I::LocalGet(1),
        I::I64ExtendI32U,
        I::I64Or,
    ])?;
    Ok(())
}
