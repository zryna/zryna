//! Clone INTO caller-owned physical frame storage with invocation-local actual progress.
use super::{
    body::{Frame, cleanup},
    invariant, records,
};
use cranelift_codegen::ir::{
    Block, InstBuilder, MemFlagsData, StackSlot, StackSlotData, StackSlotKind, Value,
    condcodes::IntCC, types,
};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{
    CloneNodeKind, VerifiedCloneNode, VerifiedCloneSource, VerifiedOperation,
};

pub(super) fn emit(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    op: VerifiedOperation<'_, '_>,
) -> Result<(), Diagnostic> {
    let clone = op.clone_frontier().ok_or_else(invariant)?;
    let source = match clone.source() {
        VerifiedCloneSource::Place(place) => c.place(b, place.id())?,
        VerifiedCloneSource::Loan(loan) => c.loan(b, loan.id())?,
    };
    let destination = c.value(b, clone.result().id())?;
    if c.p.containers().is_some() {
        let status =
            c.helpers.clone_value(c.p, b, clone.ty(), source, destination, c.invocation)?;
        if clone.frontier().may_fail() {
            super::invocation::source::operation(c, b, op, status)?;
        } else {
            b.ins().trapnz(status, cranelift_codegen::ir::TrapCode::unwrap_user(7));
        }
        return Ok(());
    }
    records::copy(
        b,
        source,
        destination,
        c.p.stored_type(clone.ty()).ok_or_else(invariant)?.size(),
    )?;
    if clone.frontier().kind() == CloneNodeKind::Copy {
        return Ok(());
    }
    if clone.prefix_failure().is_none()
        || clone.outer_failure().is_none()
        || clone.destination_place().is_none()
    {
        return Err(invariant());
    }
    let destination_place = clone.destination_place().ok_or_else(invariant)?;
    if c.f.place(destination_place.id()).ok_or_else(invariant)?.offset()
        != c.f.value(clone.result().id()).ok_or_else(invariant)?.offset()
    {
        return Err(invariant());
    }
    let cursor = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().stack_store(types::I64, zero, cursor, 0);
    let failed = b.create_block();
    b.append_block_param(failed, types::I32);
    prepare(c, b, clone.frontier(), source, destination, Progress { cursor, failed }, 0)?;
    let done = b.create_block();
    b.ins().jump(done, &[]);
    b.switch_to_block(failed);
    let status = b.block_params(failed)[0];
    rollback(c, b, clone.frontier(), source, destination, cursor, 0)?;
    cleanup(c, b, clone.outer_failure().ok_or_else(invariant)?)?;
    super::invocation::source::clone_return(b, c.transport, op, status)?;
    b.switch_to_block(done);
    Ok(())
}
fn count(c: &Frame<'_, '_, '_>, node: VerifiedCloneNode<'_, '_>) -> Result<u64, Diagnostic> {
    c.p.string_occurrences(node.ty()).ok_or_else(invariant)
}
#[derive(Clone, Copy)]
struct Progress {
    cursor: StackSlot,
    failed: Block,
}
fn prepare(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    node: VerifiedCloneNode<'_, '_>,
    source: Value,
    destination: Value,
    progress: Progress,
    base: u64,
) -> Result<(), Diagnostic> {
    let view = c.p.stored_type(node.ty()).ok_or_else(invariant)?;
    match node.kind() {
        CloneNodeKind::Copy => {}
        CloneNodeKind::String => {
            // The aggregate staging copy is not a published runtime out record.
            records::zero(b, destination, view.size())?;
            let status = super::accounting::string(
                b,
                c.invocation,
                c.runtime[1],
                c.p.string_leaf(node.ty(), true)?,
                &[source, destination],
            )?;
            let next = b.create_block();
            b.ins().brif(status, progress.failed, &[status.into()], next, &[]);
            b.switch_to_block(next);
            let initialized = b.ins().iconst(
                types::I64,
                i64::try_from(base.checked_add(1).ok_or_else(invariant)?)
                    .map_err(super::codegen)?,
            );
            b.ins().stack_store(types::I64, initialized, progress.cursor, 0);
        }
        CloneNodeKind::Struct => {
            let mut start = base;
            for ((ordinal, child), (physical, ty, offset)) in node.fields().zip(view.fields()) {
                if ordinal != physical || child.ty() != ty.index() {
                    return Err(invariant());
                }
                let s = records::add(b, source, offset)?;
                let d = records::add(b, destination, offset)?;
                prepare(c, b, child, s, d, progress, start)?;
                start = start.checked_add(count(c, child)?).ok_or_else(invariant)?;
            }
        }
        CloneNodeKind::Enum => {
            let done = b.create_block();
            let tag = b.ins().load(types::I32, MemFlagsData::new(), source, 0);
            let mut start = base;
            for (ordinal, child) in node.variants() {
                let active = b.create_block();
                let next = b.create_block();
                let test = b.ins().icmp_imm_u(IntCC::Equal, tag, i64::from(ordinal));
                b.ins().brif(test, active, &[], next, &[]);
                b.switch_to_block(active);
                if let Some(child) = child {
                    let offset = view.payload().ok_or_else(invariant)?.0;
                    let s = records::add(b, source, offset)?;
                    let d = records::add(b, destination, offset)?;
                    prepare(c, b, child, s, d, progress, start)?;
                    start = start.checked_add(count(c, child)?).ok_or_else(invariant)?;
                }
                b.ins().jump(done, &[]);
                b.switch_to_block(next);
            }
            b.ins().trap(cranelift_codegen::ir::TrapCode::unwrap_user(7));
            b.switch_to_block(done);
        }
        CloneNodeKind::FixedArray { length } => {
            let child = node.element().ok_or_else(invariant)?;
            let width = count(c, child)?;
            for index in 0..length {
                let offset = index
                    .checked_mul(view.array_stride().ok_or_else(invariant)?)
                    .ok_or_else(invariant)?;
                let s = records::add(b, source, offset)?;
                let d = records::add(b, destination, offset)?;
                let ordinal = base
                    .checked_add(index.checked_mul(width).ok_or_else(invariant)?)
                    .ok_or_else(invariant)?;
                prepare(c, b, child, s, d, progress, ordinal)?;
            }
        }
        _ => return Err(invariant()),
    }
    Ok(())
}
fn rollback(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    node: VerifiedCloneNode<'_, '_>,
    source: Value,
    destination: Value,
    cursor: StackSlot,
    base: u64,
) -> Result<(), Diagnostic> {
    let view = c.p.stored_type(node.ty()).ok_or_else(invariant)?;
    match node.kind() {
        CloneNodeKind::Copy => {}
        CloneNodeKind::String => {
            let actual = b.ins().stack_load(types::I64, types::I64, cursor, 0);
            let initialized = b.ins().icmp_imm_u(
                IntCC::UnsignedGreaterThanOrEqual,
                actual,
                i64::try_from(base.checked_add(1).ok_or_else(invariant)?)
                    .map_err(super::codegen)?,
            );
            let active = b.create_block();
            let done = b.create_block();
            b.ins().brif(initialized, active, &[], done, &[]);
            b.switch_to_block(active);
            records::release(c, b, destination, node.ty())?;
            b.ins().jump(done, &[]);
            b.switch_to_block(done);
        }
        CloneNodeKind::Struct => {
            let fields = node.fields().zip(view.fields()).collect::<Vec<_>>();
            let mut end = base.checked_add(count(c, node)?).ok_or_else(invariant)?;
            for ((_, child), (_, _, offset)) in fields.into_iter().rev() {
                end = end.checked_sub(count(c, child)?).ok_or_else(invariant)?;
                let s = records::add(b, source, offset)?;
                let d = records::add(b, destination, offset)?;
                rollback(c, b, child, s, d, cursor, end)?;
            }
        }
        CloneNodeKind::Enum => {
            let variants = node.variants().collect::<Vec<_>>();
            let mut end = base.checked_add(count(c, node)?).ok_or_else(invariant)?;
            let tag = b.ins().load(types::I32, MemFlagsData::new(), source, 0);
            let done = b.create_block();
            for (ordinal, child) in variants.into_iter().rev() {
                if let Some(child) = child {
                    end = end.checked_sub(count(c, child)?).ok_or_else(invariant)?;
                }
                let active = b.create_block();
                let next = b.create_block();
                let test = b.ins().icmp_imm_u(IntCC::Equal, tag, i64::from(ordinal));
                b.ins().brif(test, active, &[], next, &[]);
                b.switch_to_block(active);
                if let Some(child) = child {
                    let offset = view.payload().ok_or_else(invariant)?.0;
                    let s = records::add(b, source, offset)?;
                    let d = records::add(b, destination, offset)?;
                    rollback(c, b, child, s, d, cursor, end)?;
                }
                b.ins().jump(done, &[]);
                b.switch_to_block(next);
            }
            b.ins().trap(cranelift_codegen::ir::TrapCode::unwrap_user(7));
            b.switch_to_block(done);
        }
        CloneNodeKind::FixedArray { length } => {
            let child = node.element().ok_or_else(invariant)?;
            let width = count(c, child)?;
            for index in (0..length).rev() {
                let offset = index
                    .checked_mul(view.array_stride().ok_or_else(invariant)?)
                    .ok_or_else(invariant)?;
                let s = records::add(b, source, offset)?;
                let d = records::add(b, destination, offset)?;
                let ordinal = base
                    .checked_add(index.checked_mul(width).ok_or_else(invariant)?)
                    .ok_or_else(invariant)?;
                rollback(c, b, child, s, d, cursor, ordinal)?;
            }
        }
        _ => return Err(invariant()),
    }
    Ok(())
}
