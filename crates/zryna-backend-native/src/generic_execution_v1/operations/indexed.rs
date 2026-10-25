//! One exact bounds check before publishing an invocation-local element address.
use super::{Frame, invariant, scalar};
use crate::generic_execution_v1::codegen;
use cranelift_codegen::ir::{InstBuilder, MemFlagsData, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{VerifiedIndexedLoan, VerifiedOperation};
use zryna_native_mir::generic_execution_v1::{LoanAddress, TypeCategory};
pub(super) fn emit(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    op: VerifiedOperation<'_, '_>,
    view: VerifiedIndexedLoan<'_, '_>,
) -> Result<(), Diagnostic> {
    let source = if let Some(place) = view.base_place() {
        c.place(b, place.id())?
    } else {
        c.loan(b, view.base_loan().ok_or_else(invariant)?.id())?
    };
    let (data, length, stride) = geometry(c, b, view, source)?;
    let index = scalar(c, b, view.index().id())?;
    let index = b.ins().uextend(types::I64, index);
    let valid = b.ins().icmp(IntCC::UnsignedLessThan, index, length);
    let zero = b.ins().iconst(types::I32, 0);
    let bounds = b.ins().iconst(types::I32, 1);
    let status = b.ins().select(valid, zero, bounds);
    crate::generic_execution_v1::invocation::source::operation(c, b, op, status)?;
    let offset = b.ins().imul_imm_u(index, i64::try_from(stride).map_err(codegen)?);
    let pointer = b.ins().iadd(data, offset);
    let LoanAddress::Prepared { slot, offset: 0 } =
        c.f.loan(view.loan().id()).ok_or_else(invariant)?
    else {
        return Err(invariant());
    };
    let cell = b.ins().iadd_imm_u(c.base, i64::from(slot));
    let published = b.ins().store(MemFlagsData::new(), pointer, cell, 0);
    crate::generic_execution_v1::audit::indexed::check(c, b.func, view, source, status, published)
}
fn geometry(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    view: VerifiedIndexedLoan<'_, '_>,
    source: Value,
) -> Result<(Value, Value, u64), Diagnostic> {
    let layout = c.p.stored_type(view.container_type()).ok_or_else(invariant)?;
    if layout.referenced_type().ok_or_else(invariant)?.index() != view.loan().referent() {
        return Err(invariant());
    }
    match layout.category() {
        TypeCategory::FixedArray => {
            let count = layout.array_length().ok_or_else(invariant)?;
            let stride = layout.array_stride().ok_or_else(invariant)?;
            if count.checked_mul(stride).is_none_or(|n| n > layout.size()) {
                return Err(invariant());
            }
            Ok((source, b.ins().iconst(types::I64, i64::try_from(count).map_err(codegen)?), stride))
        }
        TypeCategory::Vec => {
            let element =
                c.p.containers()
                    .and_then(|p| p.element(view.container_type()))
                    .ok_or_else(invariant)?;
            if element.element().index() != view.loan().referent() {
                return Err(invariant());
            }
            let data = b.ins().load(types::I64, MemFlagsData::new(), source, 0);
            let length = b.ins().load(types::I64, MemFlagsData::new(), source, 8);
            Ok((data, length, element.stride()))
        }
        _ => Err(invariant()),
    }
}
