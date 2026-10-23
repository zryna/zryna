//! Linear reverse prefix cleanup; guards dominate every destination String lane load.
use super::{Kernel, offset, source};
use crate::generic_owned_v2::invariant;
use cranelift_codegen::ir::{InstBuilder, TrapCode, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_owned_v2::{CloneKind, CloneNode};

pub(super) fn node(
    kernel: &Kernel<'_, '_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    node: &CloneNode,
    progress: Value,
) -> Result<(), Diagnostic> {
    match node.kind() {
        CloneKind::Copy => {}
        CloneKind::String => string(kernel, b, node, progress)?,
        CloneKind::Struct => {
            for (_, child) in node.children().rev() {
                self::node(kernel, b, child.ok_or_else(invariant)?, progress)?;
            }
        }
        CloneKind::Enum => enumeration(kernel, b, node, progress)?,
    }
    Ok(())
}

fn string(
    kernel: &Kernel<'_, '_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    node: &CloneNode,
    progress: Value,
) -> Result<(), Diagnostic> {
    let active = b.create_block();
    let done = b.create_block();
    let threshold = i64::try_from(node.string_start()).map_err(|_| invariant())?;
    let initialized = b.ins().icmp_imm_u(IntCC::UnsignedGreaterThan, progress, threshold);
    b.ins().brif(initialized, active, &[], done, &[]);
    b.switch_to_block(active);
    for lane in 0..3 {
        let destination = node.begin().checked_add(lane).ok_or_else(invariant)?;
        let value =
            b.ins().stack_load(types::I64, types::I64, kernel.destination, offset(destination)?);
        b.ins().stack_store(types::I64, value, kernel.context.record, 24 + offset(lane)?);
    }
    let pointer = b.ins().stack_addr(types::I64, kernel.context.record, 24);
    let call = b.ins().call(kernel.context.runtime[2], &[pointer]);
    let status = b.inst_results(call)[0];
    b.ins().trapnz(status, TrapCode::unwrap_user(7));
    b.ins().jump(done, &[]);
    b.switch_to_block(done);
    Ok(())
}

fn enumeration(
    kernel: &Kernel<'_, '_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    node: &CloneNode,
    progress: Value,
) -> Result<(), Diagnostic> {
    let tag = *source(kernel, node)?.first().ok_or_else(invariant)?;
    let done = b.create_block();
    for (ordinal, child) in node.children() {
        let active = b.create_block();
        let next = b.create_block();
        let test = b.ins().icmp_imm_u(IntCC::Equal, tag, i64::from(ordinal));
        b.ins().brif(test, active, &[], next, &[]);
        b.switch_to_block(active);
        if let Some(child) = child {
            self::node(kernel, b, child, progress)?;
        }
        b.ins().jump(done, &[]);
        b.switch_to_block(next);
    }
    b.ins().trap(TrapCode::unwrap_user(7));
    b.switch_to_block(done);
    Ok(())
}
