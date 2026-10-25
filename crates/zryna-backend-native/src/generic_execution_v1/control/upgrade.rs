//! Atomic upgrade publishes the generated Shared only on the successful edge.
use super::{Frame, invariant, records};
use crate::generic_execution_v1::runtime;
use cranelift_codegen::ir::{Block, InstBuilder, MemFlagsData, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{VerifiedUpgradeEdge, VerifiedWeakUpgrade};

pub(super) fn emit(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    view: VerifiedWeakUpgrade<'_, '_>,
    blocks: &[Block],
    effect: zryna_native_mir::generic_execution_v1::ControlEffect<'_, '_>,
) -> Result<(), Diagnostic> {
    let source = if let Some(place) = view.source_place() {
        c.place(b, place.id())?
    } else {
        c.loan(b, view.source_loan().ok_or_else(invariant)?.id())?
    };
    let control = b.ins().load(types::I64, MemFlagsData::new(), source, 0);
    let status = runtime::control(c.p, c.runtime, b, c.invocation, effect, &[control])?;
    let expired = b.ins().icmp_imm_u(IntCC::Equal, status, 5);
    let zero = b.ins().iconst(types::I32, 0);
    let failed = b.ins().select(expired, zero, status);
    crate::generic_execution_v1::invocation::source::upgrade(c, b, view, failed)?;
    let success = b.create_block();
    let expiration = b.create_block();
    let acquired = b.ins().icmp_imm_u(IntCC::Equal, status, 0);
    b.ins().brif(acquired, success, &[], expiration, &[]);
    b.switch_to_block(success);
    transfer(c, b, view.success(), Some((view.success_binding().id(), control)), blocks)?;
    b.switch_to_block(expiration);
    transfer(c, b, view.expired(), None, blocks)
}

fn transfer(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    edge: VerifiedUpgradeEdge<'_, '_>,
    binding: Option<(u32, Value)>,
    blocks: &[Block],
) -> Result<(), Diagnostic> {
    let target = edge.target();
    let skip = usize::from(binding.is_some());
    if edge.arguments().len()
        != target.value_parameters().len().checked_sub(skip).ok_or_else(invariant)?
    {
        return Err(invariant());
    }
    let capacity = target.value_parameters().skip(skip).try_fold(0usize, |count, parameter| {
        count.checked_add(records::chunks(c.size(parameter.ty())?).count()).ok_or_else(invariant)
    })?;
    let mut pending = Vec::new();
    pending.try_reserve_exact(capacity).map_err(crate::generic_execution_v1::codegen)?;
    for (source, destination) in edge.arguments().zip(target.value_parameters().skip(skip)) {
        let from = c.value(b, source.id())?;
        let to = c.value(b, destination.id())?;
        for (offset, ty) in records::chunks(c.size(source.ty())?) {
            let value = b.ins().load(
                ty,
                MemFlagsData::new(),
                from,
                super::super::body::checked_offset(offset)?,
            );
            pending.push((to, offset, value));
        }
    }
    // Snapshot ordinary arguments before any generated or ordinary destination is overwritten.
    if let Some((id, control)) = binding {
        let destination = c.value(b, id)?;
        b.ins().store(MemFlagsData::new(), control, destination, 0);
    }
    for (destination, offset, value) in pending {
        b.ins().store(
            MemFlagsData::new(),
            value,
            destination,
            super::super::body::checked_offset(offset)?,
        );
    }
    b.ins().jump(*blocks.get(target.id() as usize).ok_or_else(invariant)?, &[]);
    Ok(())
}
