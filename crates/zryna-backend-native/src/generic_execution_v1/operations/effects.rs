//! Private calls, literal acquisition and checked dynamic Copy indexing.
use super::{
    super::{
        body::{Frame, failure},
        invariant, records,
    },
    copy_result, scalar, source_type,
};
use crate::generic_execution_v1::codegen;
use cranelift_codegen::ir::{
    InstBuilder, StackSlotData, StackSlotKind, Value, condcodes::IntCC, types,
};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{VerifiedOperation, raw};
pub(super) fn emit(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    op: VerifiedOperation<'_, '_>,
    result: Option<Value>,
) -> Result<(), Diagnostic> {
    match op.kind() {
        raw::Operation::ClosedGenericCall { arguments, .. }
        | raw::Operation::SourceCall { arguments, .. } => {
            let destination = result.unwrap_or(c.base);
            let mut parameters = vec![destination];
            for argument in arguments {
                parameters.push(match argument {
                    raw::CallArgument::Value(id) => c.value(b, *id)?,
                    raw::CallArgument::Loan(id) => c.loan(b, *id)?,
                });
            }
            parameters.push(c.invocation);
            let target = op.call_target().ok_or_else(invariant)?.index();
            let callee = *c.callees.get(&target).ok_or_else(invariant)?;
            let call = b.ins().call(callee, &parameters);
            let status = b.inst_results(call)[0];
            if op.failure().is_some() {
                failure(c, b, op.failure(), status)?;
            } else {
                b.ins().trapnz(status, cranelift_codegen::ir::TrapCode::unwrap_user(7));
            }
        }
        raw::Operation::StringFromUtf8 { bytes, .. } => {
            let slot = b.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                u32::try_from(bytes.len().max(1)).map_err(codegen)?,
                0,
            ));
            for (index, byte) in bytes.iter().enumerate() {
                let value = b.ins().iconst(types::I8, i64::from(*byte));
                b.ins().stack_store(
                    types::I64,
                    value,
                    slot,
                    i32::try_from(index).map_err(codegen)?,
                );
            }
            let source = b.ins().stack_addr(types::I64, slot, 0);
            let length = b.ins().iconst(types::I64, i64::try_from(bytes.len()).map_err(codegen)?);
            let destination = result.ok_or_else(invariant)?;
            let value = op.result().ok_or_else(invariant)?;
            records::zero(b, destination, c.size(value.ty())?)?;
            let status = super::super::accounting::string(
                b,
                c.invocation,
                c.runtime[0],
                c.p.invocation_plan().source_string(op)?,
                &[source, length, destination],
            )?;
            super::super::invocation::source::operation(c, b, op, status)?;
        }
        raw::Operation::IndexCopy { base, index, .. } => {
            if c.p.containers().is_some() {
                return super::super::containers::index(
                    c,
                    op,
                    b,
                    op.index_copy().ok_or_else(invariant)?,
                );
            }
            let source = c.source(b, *base)?;
            let ty = source_type(c, *base)?;
            let view = c.p.stored_type(ty).ok_or_else(invariant)?;
            let index = scalar(c, b, *index)?;
            let index = b.ins().uextend(types::I64, index);
            let length = view.array_length().ok_or_else(invariant)?;
            let valid = b.ins().icmp_imm_u(
                IntCC::UnsignedLessThan,
                index,
                i64::try_from(length).map_err(codegen)?,
            );
            let zero = b_zero(b);
            let one = b_one(b);
            let status = b.ins().select(valid, zero, one);
            super::super::invocation::source::operation(c, b, op, status)?;
            let offset = b.ins().imul_imm_u(
                index,
                i64::try_from(view.array_stride().ok_or_else(invariant)?).map_err(codegen)?,
            );
            let source = b.ins().iadd(source, offset);
            copy_result(c, b, op, source)?;
        }
        _ => return Err(invariant()),
    }
    Ok(())
}
fn b_zero(b: &mut FunctionBuilder<'_>) -> Value {
    b.ins().iconst(types::I32, 0)
}
fn b_one(b: &mut FunctionBuilder<'_>) -> Value {
    b.ins().iconst(types::I32, 1)
}
