//! Explicit edge transfers, selected payload bindings and exact reverse exit cleanup.
use super::{
    body::{Frame, cleanup},
    invariant, operations, records,
};
use cranelift_codegen::ir::{Block, InstBuilder, MemFlagsData, TrapCode, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{VerifiedBlock, raw};
mod upgrade;
pub(super) fn emit(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    block: VerifiedBlock<'_, '_>,
    blocks: &[Block],
    block_values: &[Vec<u32>],
) -> Result<(), Diagnostic> {
    match block.terminator() {
        raw::Terminator::Return { value, .. } => {
            if let Some(id) = value {
                let source = c.value(b, *id)?;
                records::copy(b, source, c.parameters[0], c.size(c.f.source().result_type())?)?;
            }
            cleanup(c, b, block.exit_cleanup().ok_or_else(invariant)?)?;
            let status = b.ins().iconst(types::I32, 0);
            b.ins().return_(&[status]);
        }
        raw::Terminator::Trap { identity, .. } => {
            cleanup(c, b, block.exit_cleanup().ok_or_else(invariant)?)?;
            let code = match identity {
                raw::TrapIdentity::Bounds => 1,
                raw::TrapIdentity::Allocation => 2,
                raw::TrapIdentity::Capacity => 3,
                raw::TrapIdentity::Refcount => 4,
                raw::TrapIdentity::Utf8 => 5,
            };
            let status = b.ins().iconst(types::I32, code);
            b.ins().return_(&[status]);
        }
        raw::Terminator::Jump(edge) => edge_transfer(c, b, edge, 0, blocks, block_values)?,
        raw::Terminator::Branch { condition, yes, no } => {
            let condition = operations::scalar(c, b, *condition)?;
            let selected = b.create_block();
            let other = b.create_block();
            b.ins().brif(condition, selected, &[], other, &[]);
            b.switch_to_block(selected);
            edge_transfer(c, b, yes, 0, blocks, block_values)?;
            b.switch_to_block(other);
            edge_transfer(c, b, no, 0, blocks, block_values)?;
        }
        raw::Terminator::ClosedEnumMatch { ty, scrutinee, mode, arms } => {
            if *mode != raw::MatchMode::Value {
                return Err(invariant());
            }
            let source = c.source(b, *scrutinee)?;
            let view = c.p.stored_type(*ty).ok_or_else(invariant)?;
            let tag = b.ins().load(types::I32, MemFlagsData::new(), source, 0);
            for arm in arms {
                let active = b.create_block();
                let next = b.create_block();
                let test = b.ins().icmp_imm_u(IntCC::Equal, tag, i64::from(arm.ordinal));
                b.ins().brif(test, active, &[], next, &[]);
                b.switch_to_block(active);
                let skip = if let Some(binding) = &arm.binding {
                    let raw::MatchBinding::Value(value) = binding else {
                        return Err(invariant());
                    };
                    let destination = c.value(b, value.id)?;
                    let payload = records::add(b, source, view.payload().ok_or_else(invariant)?.0)?;
                    records::copy(b, payload, destination, c.size(value.ty)?)?;
                    1
                } else {
                    0
                };
                edge_transfer(c, b, &arm.edge, skip, blocks, block_values)?;
                b.switch_to_block(next);
            }
            b.ins().trap(TrapCode::unwrap_user(7));
        }
        raw::Terminator::WeakUpgradeBranch { .. } => {
            upgrade::emit(c, b, block.weak_upgrade().ok_or_else(invariant)?, blocks)?;
        }
    }
    Ok(())
}
fn edge_transfer(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    edge: &raw::Edge,
    skip: usize,
    blocks: &[Block],
    block_values: &[Vec<u32>],
) -> Result<(), Diagnostic> {
    let parameters = block_values.get(edge.target as usize).ok_or_else(invariant)?;
    // Snapshot all incoming bytes before the first destination write, retaining parallel moves.
    let mut pending = Vec::new();
    for (source, destination) in edge.arguments.iter().zip(parameters.iter().skip(skip)) {
        let source_record = c.f.value(*source).ok_or_else(invariant)?;
        let source = c.record(b, source_record);
        let destination = c.value(b, *destination)?;
        for (offset, ty) in records::chunks(c.size(source_record.ty())?) {
            let value =
                b.ins().load(ty, MemFlagsData::new(), source, super::body::checked_offset(offset)?);
            pending.push((destination, offset, value));
        }
    }
    for (destination, offset, value) in pending {
        b.ins().store(
            MemFlagsData::new(),
            value,
            destination,
            super::body::checked_offset(offset)?,
        );
    }
    b.ins().jump(*blocks.get(edge.target as usize).ok_or_else(invariant)?, &[]);
    Ok(())
}
