//! Shared invocation adapters around canonical String kernels; allocation factoring has no extra debit.
use super::{Body, Diagnostic, FunctionCode, I, Model, canonical_failure, model};
use wasm_encoder::BlockType;
mod controls;
mod counters;
mod vectors;
use counters::{admit, context, funded, zero};
pub(super) fn append(m: &Model<'_, '_, '_>, out: &mut Vec<FunctionCode>) -> Result<(), Diagnostic> {
    if m.accounted() {
        for role in 0..3 + u32::from(m.concat) {
            out.push(adapter(m, role)?);
        }
        vectors::append(m, out)?;
        controls::append(m, out)?;
    }
    Ok(())
}
fn adapter(m: &Model<'_, '_, '_>, role: u32) -> Result<FunctionCode, Diagnostic> {
    if role == 2 {
        return release(m);
    }
    let parameters = if role == 1 { 4 } else { 3 };
    let status = parameters + 3;
    let possible = parameters + 1;
    let mut b = Body::new(m, if role == 3 { 5 } else { 4 })?;
    prepare(&mut b, role, parameters)?;
    for param in 0..parameters {
        b.op(&I::LocalGet(param))?;
    }
    b.ops([
        I::Call(if role == 0 {
            model::STRING_FROM
        } else if role == 1 {
            model::STRING_CLONE
        } else {
            m.concat_index()
        }),
        I::LocalSet(status),
        I::LocalGet(status),
        I::If(BlockType::Empty),
        I::GlobalGet(12),
        I::LocalGet(possible),
        I::I32Sub,
        I::GlobalSet(12),
    ])?;
    complete(&mut b, role, parameters)?;
    FunctionCode::new(parameters, 1, if role == 3 { 5 } else { 4 }, b)
}
fn prepare(b: &mut Body<'_>, role: u32, parameters: u32) -> Result<(), Diagnostic> {
    let length = parameters;
    let possible = parameters + 1;
    let positive = parameters + 2;
    if role == 3 {
        b.load(0, 4)?;
        b.op(&I::LocalSet(7))?;
        b.load(1, 4)?;
        b.ops([
            I::LocalGet(7),
            I::I32Add,
            I::LocalTee(length),
            I::LocalGet(7),
            I::I32GeU,
            I::LocalGet(length),
            I::I32Const(2_147_483_647),
            I::I32LeU,
            I::I32And,
            I::LocalSet(possible),
        ])?;
        b.load(1, 4)?;
        b.ops([I::LocalGet(7), I::I32Or, I::LocalSet(7)])?;
    } else {
        b.ops([
            I::LocalGet(1),
            I::LocalTee(length),
            I::I32Const(2_147_483_647),
            I::I32LeU,
            I::LocalSet(possible),
        ])?;
    }
    b.ops([
        I::LocalGet(length),
        I::I32Eqz,
        I::I32Eqz,
        I::LocalGet(possible),
        I::I32And,
        I::LocalSet(positive),
    ])?;
    let output = if role == 1 { 3 } else { 2 };
    zero(b, output)?;
    context(b)?;
    if role == 1 || role == 3 {
        funded(b, if role == 3 { 7 } else { length })?;
    }
    admit(b, possible, positive)?;
    Ok(())
}
fn complete(b: &mut Body<'_>, role: u32, parameters: u32) -> Result<(), Diagnostic> {
    let possible = parameters + 1;
    let status = parameters + 3;
    let output = if role == 1 { 3 } else { 2 };
    zero(b, output)?;
    // Only statuses in this canonical role's declared set may propagate.
    b.ops([
        I::LocalGet(status),
        I::I32Const(1),
        I::I32Eq,
        I::LocalGet(status),
        I::I32Const(2),
        I::I32Eq,
        I::I32Or,
        I::LocalGet(status),
        I::I32Const(255),
        I::I32Eq,
        I::I32Or,
    ])?;
    if role == 0 {
        b.ops([I::LocalGet(status), I::I32Const(4), I::I32Eq, I::I32Or])?;
    }
    b.ops([I::I32Eqz, I::If(BlockType::Empty)])?;
    canonical_failure(b, 255, 255)?;
    b.ops([
        I::End,
        I::LocalGet(status),
        I::Return,
        I::End,
        I::LocalGet(possible),
        I::I32Eqz,
        I::If(BlockType::Empty),
    ])?;
    canonical_failure(b, 255, 255)?;
    b.ops([I::End, I::I32Const(0)])?;
    Ok(())
}
fn release(m: &Model<'_, '_, '_>) -> Result<FunctionCode, Diagnostic> {
    let mut b = Body::new(m, 1)?;
    context(&mut b)?;
    funded(&mut b, 1)?;
    b.ops([
        I::GlobalGet(12),
        I::I32Const(1),
        I::I32Sub,
        I::GlobalSet(12),
        I::GlobalGet(11),
        I::I32Const(1),
        I::I32Add,
        I::GlobalSet(11),
        I::LocalGet(0),
        I::LocalGet(1),
        I::LocalGet(2),
        I::Call(model::STRING_RELEASE),
        I::LocalSet(3),
        I::LocalGet(3),
        I::If(BlockType::Empty),
    ])?;
    canonical_failure(&mut b, 255, 255)?;
    b.ops([I::End, I::I32Const(0)])?;
    FunctionCode::new(3, 1, 1, b)
}
