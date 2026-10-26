//! Exact scalar ABI wrappers and boolean carrier guards for owned core Wasm.

use super::super::budget;
use super::{Bytes, Layout, Op};
use wasm_encoder::BlockType;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::raw;

pub(super) fn wrapper(layout: &Layout<'_, '_>, index: usize) -> Result<Bytes, Diagnostic> {
    let function = &layout.program.functions()[index];
    let mut body = Bytes::new();
    body.extend(&[1])?;
    body.u32(1)?;
    body.extend(&[0x7f])?;
    for (i, p) in function.parameters.iter().enumerate() {
        if *p == raw::Type::Stored(0) {
            bool_guard(&mut body, &Op::LocalGet(u32::try_from(i).map_err(|_| budget())?))?;
        }
    }
    body.op(&Op::GlobalGet(layout.return_lanes))?;
    body.op(&Op::If(BlockType::Empty))?;
    body.op(&Op::Unreachable)?;
    body.op(&Op::End)?;
    body.op(&Op::I32Const(1))?;
    body.op(&Op::GlobalSet(layout.return_lanes))?;
    for i in 0..function.parameters.len() {
        body.op(&Op::LocalGet(u32::try_from(i).map_err(|_| budget())?))?;
    }
    body.op(&Op::Call(u32::try_from(index).map_err(|_| budget())? + 3))?;
    body.op(&Op::LocalSet(u32::try_from(function.parameters.len()).map_err(|_| budget())?))?;
    body.op(&Op::I32Const(0))?;
    body.op(&Op::GlobalSet(layout.return_lanes))?;
    body.op(&Op::LocalGet(u32::try_from(function.parameters.len()).map_err(|_| budget())?))?;
    body.op(&Op::If(BlockType::Empty))?;
    body.op(&Op::Unreachable)?;
    body.op(&Op::End)?;
    if function.result == raw::Type::Stored(0) {
        bool_guard(&mut body, &Op::GlobalGet(0))?;
    }
    body.op(&Op::GlobalGet(0))?;
    body.op(&Op::End)?;
    Ok(body)
}

fn bool_guard(body: &mut Bytes, carrier: &Op<'_>) -> Result<(), Diagnostic> {
    body.op(carrier)?;
    body.op(&Op::I32Const(0))?;
    body.op(&Op::I32Eq)?;
    body.op(carrier)?;
    body.op(&Op::I32Const(1))?;
    body.op(&Op::I32Eq)?;
    body.op(&Op::I32Or)?;
    body.op(&Op::If(BlockType::Empty))?;
    body.op(&Op::Else)?;
    body.op(&Op::Unreachable)?;
    body.op(&Op::End)
}
