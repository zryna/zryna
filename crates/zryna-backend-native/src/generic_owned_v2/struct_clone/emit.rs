//! Forward declaration-order traversal, checking String status before reading its out record.
use super::{Kernel, offset, source};
use crate::generic_owned_v2::{body::store, codegen, invariant};
use cranelift_codegen::ir::{InstBuilder, TrapCode, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_owned_v2::{CloneKind, CloneNode};

pub(super) fn node(
    kernel: &Kernel<'_, '_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    node: &CloneNode,
) -> Result<(), Diagnostic> {
    match node.kind() {
        CloneKind::Copy => {
            for (lane, value) in source(kernel, node)?.iter().enumerate() {
                let lane = node
                    .begin()
                    .checked_add(u32::try_from(lane).map_err(codegen)?)
                    .ok_or_else(invariant)?;
                b.ins().stack_store(types::I64, *value, kernel.destination, offset(lane)?);
            }
        }
        CloneKind::String => string(kernel, b, node)?,
        CloneKind::Struct => {
            for (_, child) in node.children() {
                self::node(kernel, b, child.ok_or_else(invariant)?)?;
            }
        }
        CloneKind::Enum => enumeration(kernel, b, node)?,
    }
    Ok(())
}

fn string(
    kernel: &Kernel<'_, '_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    node: &CloneNode,
) -> Result<(), Diagnostic> {
    let record = kernel.context.record;
    let input = b.ins().stack_addr(types::I64, record, 24);
    store(b, input, source(kernel, node)?)?;
    let output = b.ins().stack_addr(types::I64, record, 0);
    let call = b.ins().call(kernel.context.runtime[1], &[input, output]);
    let status = b.inst_results(call)[0];
    let success = b.create_block();
    b.ins().brif(status, kernel.failed, &[status.into()], success, &[]);
    b.switch_to_block(success);
    for lane in 0..3 {
        let value = b.ins().stack_load(types::I64, types::I64, record, offset(lane)?);
        let destination = node.begin().checked_add(lane).ok_or_else(invariant)?;
        b.ins().stack_store(types::I64, value, kernel.destination, offset(destination)?);
    }
    let progress = node.string_start().checked_add(1).ok_or_else(invariant)?;
    let progress = b.ins().iconst(types::I64, i64::try_from(progress).map_err(codegen)?);
    b.ins().stack_store(types::I64, progress, kernel.cursor, 0);
    Ok(())
}

fn enumeration(
    kernel: &Kernel<'_, '_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    node: &CloneNode,
) -> Result<(), Diagnostic> {
    let tag = *source(kernel, node)?.first().ok_or_else(invariant)?;
    b.ins().stack_store(types::I64, tag, kernel.destination, offset(node.begin())?);
    let done = b.create_block();
    for (ordinal, child) in node.children() {
        let active = b.create_block();
        let next = b.create_block();
        let test = b.ins().icmp_imm_u(IntCC::Equal, tag, i64::from(ordinal));
        b.ins().brif(test, active, &[], next, &[]);
        b.switch_to_block(active);
        if let Some(child) = child {
            self::node(kernel, b, child)?;
        }
        b.ins().jump(done, &[]);
        b.switch_to_block(next);
    }
    b.ins().trap(TrapCode::unwrap_user(7));
    b.switch_to_block(done);
    Ok(())
}
