//! Prepared RHS remains owned until successful reserve and infallible element commit.
use super::{Frame, O, codegen, failure, invariant, records, runtime};
use cranelift_codegen::ir::{
    InstBuilder, MemFlagsData, StackSlotData, StackSlotKind, condcodes::IntCC, types,
};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::VerifiedVecPush;

pub(super) fn emit(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    view: VerifiedVecPush<'_, '_>,
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
    failure(c, b, Some(view.outer_failure()), status)?;
    // Reserve requires distinct caller-zero output; failure never replaces the original handle.
    let slot = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 24, 3));
    let out = b.ins().stack_addr(types::I64, slot, 0);
    records::zero(b, out, 24)?;
    let element = b.ins().iconst(types::I32, i64::from(metadata.element().index()));
    let status =
        runtime::call(c.p, c.runtime, b, O::VecReserve, &[element, target, required, out])?;
    failure(c, b, Some(view.outer_failure()), status)?;
    let data = b.ins().load(types::I64, MemFlagsData::new(), out, 0);
    let offset = b.ins().imul_imm_u(length, i64::try_from(metadata.stride()).map_err(codegen)?);
    let destination = b.ins().iadd(data, offset);
    let source = c.value(b, view.value().id())?;
    records::copy(b, source, destination, c.size(view.value().ty())?)?;
    b.ins().store(MemFlagsData::new(), required, out, 8);
    records::copy(b, out, target, 24)
}
