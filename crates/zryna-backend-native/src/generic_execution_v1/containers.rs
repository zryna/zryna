//! Checked preparations commit pending owners only after existing runtime acquisition succeeds.
use super::{body::Frame, codegen, invariant, operations::scalar, records, runtime};
use cranelift_codegen::ir::{
    InstBuilder, MemFlagsData, StackSlotData, StackSlotKind, condcodes::IntCC, types,
};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{
    VerifiedIndexCopy, VerifiedOperation, VerifiedSharedConstruct, VerifiedVecConstruct,
    VerifiedWeakDowngrade,
};
use zryna_native_mir::generic_execution_v1::TypeCategory;
mod push;
pub(super) fn emit(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    op: VerifiedOperation<'_, '_>,
) -> Result<(), Diagnostic> {
    if let Some(view) = op.vec_construct() {
        vector(c, b, op, view, c.p.source_vector(op)?)
    } else if let Some(view) = op.shared_construct() {
        shared(c, b, op, view, c.p.source_control(op)?)
    } else if let Some(view) = op.vec_push() {
        push::emit(c, b, op, view, c.p.source_vector(op)?)
    } else if let Some(view) = op.weak_downgrade() {
        weak(c, b, op, view, c.p.source_control(op)?)
    } else {
        Err(invariant())
    }
}
fn vector(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    op: VerifiedOperation<'_, '_>,
    view: VerifiedVecConstruct<'_, '_>,
    effect: zryna_native_mir::generic_execution_v1::VectorEffect<'_, '_>,
) -> Result<(), Diagnostic> {
    let metadata = c.p.containers().and_then(|p| p.element(view.ty())).ok_or_else(invariant)?;
    if metadata.element().index() != view.element_type() {
        return Err(invariant());
    }
    let destination = c.value(b, view.result().id())?;
    let element = b.ins().iconst(types::I32, i64::from(metadata.element().index()));
    let length = b.ins().iconst(types::I64, i64::try_from(view.elements().len()).map_err(codegen)?);
    records::zero(b, destination, c.size(view.result().ty())?)?;
    let status =
        runtime::vector(c.p, c.runtime, b, c.invocation, effect, &[element, length, destination])?
            .status;
    super::invocation::source::operation(c, b, op, status)?;
    let data = b.ins().load(types::I64, MemFlagsData::new(), destination, 0);
    for (index, value) in view.elements().enumerate() {
        let source = c.value(b, value.id())?;
        let offset = (index as u64).checked_mul(metadata.stride()).ok_or_else(invariant)?;
        let target = records::add(b, data, offset)?;
        records::copy(b, source, target, c.size(value.ty())?)?;
    }
    b.ins().store(MemFlagsData::new(), length, destination, 8);
    Ok(())
}
fn shared(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    op: VerifiedOperation<'_, '_>,
    view: VerifiedSharedConstruct<'_, '_>,
    effect: zryna_native_mir::generic_execution_v1::ControlEffect<'_, '_>,
) -> Result<(), Diagnostic> {
    let metadata = c.p.containers().and_then(|p| p.control(view.ty())).ok_or_else(invariant)?;
    if metadata.payload().index() != view.referent_type() {
        return Err(invariant());
    }
    let slot = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
    let out = b.ins().stack_addr(types::I64, slot, 0);
    records::zero(b, out, 8)?;
    let size = b.ins().iconst(types::I64, i64::try_from(metadata.size()).map_err(codegen)?);
    let alignment =
        b.ins().iconst(types::I32, i64::try_from(metadata.alignment()).map_err(codegen)?);
    let status =
        runtime::control(c.p, c.runtime, b, c.invocation, effect, &[size, alignment, out])?;
    super::invocation::source::operation(c, b, op, status)?;
    let control = b.ins().load(types::I64, MemFlagsData::new(), out, 0);
    let one = b.ins().iconst(types::I32, 1);
    b.ins().store(MemFlagsData::new(), one, control, 0);
    b.ins().store(MemFlagsData::new(), one, control, 4);
    let payload = records::add(b, control, metadata.payload_offset())?;
    let source = c.value(b, view.payload().id())?;
    records::copy(b, source, payload, c.size(view.payload().ty())?)?;
    let destination = c.value(b, view.result().id())?;
    b.ins().store(MemFlagsData::new(), control, destination, 0);
    Ok(())
}
fn weak(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    op: VerifiedOperation<'_, '_>,
    view: VerifiedWeakDowngrade<'_, '_>,
    effect: zryna_native_mir::generic_execution_v1::ControlEffect<'_, '_>,
) -> Result<(), Diagnostic> {
    let source = if let Some(place) = view.source_place() {
        c.place(b, place.id())?
    } else {
        c.loan(b, view.source_loan().ok_or_else(invariant)?.id())?
    };
    let control = b.ins().load(types::I64, MemFlagsData::new(), source, 0);
    let status = runtime::control(c.p, c.runtime, b, c.invocation, effect, &[control])?;
    super::invocation::source::operation(c, b, op, status)?;
    let destination = c.value(b, view.result().id())?;
    b.ins().store(MemFlagsData::new(), control, destination, 0);
    Ok(())
}
pub(super) fn index(
    c: &Frame<'_, '_, '_>,
    op: VerifiedOperation<'_, '_>,
    b: &mut FunctionBuilder<'_>,
    view: VerifiedIndexCopy<'_, '_>,
) -> Result<(), Diagnostic> {
    let source = if let Some(place) = view.source_place() {
        c.place(b, place.id())?
    } else {
        c.loan(b, view.source_loan().ok_or_else(invariant)?.id())?
    };
    let layout = c.p.stored_type(view.container_type()).ok_or_else(invariant)?;
    let (data, length, stride) = match layout.category() {
        TypeCategory::FixedArray => {
            let length = b.ins().iconst(
                types::I64,
                i64::try_from(layout.array_length().ok_or_else(invariant)?).map_err(codegen)?,
            );
            (source, length, layout.array_stride().ok_or_else(invariant)?)
        }
        TypeCategory::Vec => {
            let metadata =
                c.p.containers()
                    .and_then(|p| p.element(view.container_type()))
                    .ok_or_else(invariant)?;
            (
                b.ins().load(types::I64, MemFlagsData::new(), source, 0),
                b.ins().load(types::I64, MemFlagsData::new(), source, 8),
                metadata.stride(),
            )
        }
        _ => return Err(invariant()),
    };
    let index = scalar(c, b, view.index().id())?;
    let index = b.ins().uextend(types::I64, index);
    let valid = b.ins().icmp(IntCC::UnsignedLessThan, index, length);
    let zero = b.ins().iconst(types::I32, 0);
    let bounds = b.ins().iconst(types::I32, 1);
    let status = b.ins().select(valid, zero, bounds);
    super::invocation::source::operation(c, b, op, status)?;
    let offset = b.ins().imul_imm_u(index, i64::try_from(stride).map_err(codegen)?);
    let source = b.ins().iadd(data, offset);
    let destination = c.value(b, view.result().id())?;
    records::copy(b, source, destination, c.size(view.result().ty())?)
}
