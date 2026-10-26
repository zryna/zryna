//! Prepared RHS remains owned until successful reserve and infallible element commit.
use super::{Frame, codegen, invariant, records, runtime};
use cranelift_codegen::ir::{
    InstBuilder, MemFlagsData, StackSlotData, StackSlotKind, condcodes::IntCC, types,
};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{VerifiedOperation, VerifiedVecPush};

pub(super) fn emit(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    op: VerifiedOperation<'_, '_>,
    view: VerifiedVecPush<'_, '_>,
    effect: zryna_native_mir::generic_execution_v1::VectorEffect<'_, '_>,
) -> Result<(), Diagnostic> {
    let metadata = c.p.containers().and_then(|p| p.element(view.ty())).ok_or_else(invariant)?;
    if metadata.element().index() != view.element_type() {
        return Err(invariant());
    }
    let target = if let Some(place) = view.target_place() {
        c.place(b, place.id())?
    } else {
        c.loan(b, view.target_loan().ok_or_else(invariant)?.id())?
    };
    // Resolve/reload the target only after source-authorized RHS preparation has completed.
    let length = b.ins().load(types::I64, MemFlagsData::new(), target, 8);
    let required = b.ins().iadd_imm_u(length, 1);
    let overflow = b.ins().icmp(IntCC::UnsignedLessThan, required, length);
    let zero = b.ins().iconst(types::I32, 0);
    let capacity = b.ins().iconst(types::I32, 2);
    let status = b.ins().select(overflow, capacity, zero);
    super::super::invocation::source::overflow(c, b, view, status)?;
    // Reserve requires distinct caller-zero output; failure never replaces the original handle.
    let slot = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 24, 3));
    let out = b.ins().stack_addr(types::I64, slot, 0);
    records::zero(b, out, 24)?;
    let element = b.ins().iconst(types::I32, i64::from(metadata.element().index()));
    // Normalize only this source Push's empty positive request. The old runtime's
    // private doubling then produces the ABI's first capacity4 in one canonical call.
    let old_capacity = b.ins().load(types::I64, MemFlagsData::new(), target, 16);
    let four = b.ins().iconst(types::I64, 4);
    let small = b.ins().icmp(IntCC::UnsignedLessThan, required, four);
    let minimum = b.ins().select(small, four, required);
    let empty = b.ins().icmp_imm_u(IntCC::Equal, old_capacity, 0);
    let request = b.ins().select(empty, minimum, required);
    let guarded =
        runtime::vector(c.p, c.runtime, b, c.invocation, effect, &[element, target, request, out])?;
    let status = guarded.status;
    super::super::invocation::source::operation(c, b, op, status)?;
    let data = b.ins().load(types::I64, MemFlagsData::new(), out, 0);
    let offset = b.ins().imul_imm_u(length, i64::try_from(metadata.stride()).map_err(codegen)?);
    let destination = b.ins().iadd(data, offset);
    let source = c.value(b, view.value().id())?;
    records::copy(b, source, destination, c.size(view.value().ty())?)?;
    let published = b.ins().store(MemFlagsData::new(), required, out, 8);
    super::super::audit::push_request::check(
        c,
        b.func,
        view,
        target,
        guarded.canonical,
        published,
    )?;
    records::copy(b, out, target, 24)
}
