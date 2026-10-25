//! Baked typed invocation; source scalar is unpublished until cleanup and lease reset.
use super::plan::Plan;
use crate::generic_execution_v1::{accounting, codegen, recursive};
use cranelift_codegen::{
    Context,
    ir::{
        AbiParam, FuncRef, InstBuilder, MemFlagsData, Signature, StackSlotData, StackSlotKind,
        TrapCode, condcodes::IntCC, types,
    },
    isa::CallConv,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use zryna_abi::{ScalarType, ScalarValue};
use zryna_diagnostics::Diagnostic;
pub(in crate::generic_execution_v1) fn signature() -> Signature {
    let mut s = Signature::new(CallConv::SystemV);
    s.params.push(AbiParam::new(types::I64));
    s.returns.push(AbiParam::new(types::I32));
    s
}
pub(in crate::generic_execution_v1) fn build(
    plan: &Plan<'_>,
    callee: FuncRef,
    context: &mut Context,
    bc: &mut FunctionBuilderContext,
    config: cranelift_codegen::isa::TargetFrontendConfig,
    tls: Option<FuncRef>,
) -> Result<(), Diagnostic> {
    let mut b = FunctionBuilder::new(&mut context.func, bc);
    let block = b.create_block();
    b.append_block_params_for_function_params(block);
    b.switch_to_block(block);
    b.seal_block(block);
    let host = b.block_params(block)[0];
    let zero = b.ins().iconst(types::I32, 0);
    b.ins().store(MemFlagsData::new(), zero, host, 0);
    let output = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
    let out = b.ins().stack_addr(types::I64, output, 0);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().store(MemFlagsData::new(), zero, out, 0);
    let mut arguments = Vec::new();
    arguments.try_reserve_exact(plan.request.arguments().len() + 2).map_err(codegen)?;
    arguments.push(out);
    for value in plan.request.arguments() {
        let slot = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
        let address = b.ins().stack_addr(types::I64, slot, 0);
        let zero = b.ins().iconst(types::I64, 0);
        b.ins().store(MemFlagsData::new(), zero, address, 0);
        let value = match value {
            ScalarValue::Bool(value) => b.ins().iconst(types::I8, i64::from(*value)),
            ScalarValue::I32(value) => b.ins().iconst(types::I32, i64::from(*value)),
        };
        b.ins().store(MemFlagsData::new(), value, address, 0);
        arguments.push(address);
    }
    let slot = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 32, 3));
    let ctx = b.ins().stack_addr(types::I64, slot, 0);
    let zero = b.ins().iconst(types::I64, 0);
    for offset in [0, 8, 16, 24] {
        b.ins().store(MemFlagsData::new(), zero, ctx, offset);
    }
    let lease = tls.map(|getter| recursive::lifecycle::begin(&mut b, getter, ctx));
    arguments.push(ctx);
    let call = b.ins().call(callee, &arguments);
    let status = b.inst_results(call)[0];
    if let Some(base) = lease {
        recursive::lifecycle::completed(&mut b, base, ctx);
    }
    accounting::finish(&mut b, ctx);
    if let Some(base) = lease {
        recursive::lifecycle::end(&mut b, base);
    }
    let out_of_range = b.ins().icmp_imm_u(IntCC::UnsignedGreaterThan, status, 5);
    let not_invariant = b.ins().icmp_imm_u(IntCC::NotEqual, status, 255);
    let invalid = b.ins().band(out_of_range, not_invariant);
    b.ins().trapnz(invalid, TrapCode::unwrap_user(7));
    let value = if plan.request.export().result() == ScalarType::Bool {
        let value = b.ins().load(types::I8, MemFlagsData::new(), out, 0);
        let value = b.ins().uextend(types::I32, value);
        let invalid = b.ins().icmp_imm_u(IntCC::UnsignedGreaterThan, value, 1);
        b.ins().trapnz(invalid, TrapCode::unwrap_user(7));
        value
    } else {
        b.ins().load(types::I32, MemFlagsData::new(), out, 0)
    };
    let success = b.ins().icmp_imm_u(IntCC::Equal, status, 0);
    let zero = b.ins().iconst(types::I32, 0);
    let value = b.ins().select(success, value, zero);
    b.ins().store(MemFlagsData::new(), value, host, 0);
    b.ins().return_(&[status]);
    recursive::check::wrapper::invocation::check(b.func, &plan.request, callee, tls)?;
    b.finalize(config);
    Ok(())
}

#[cfg(test)]
pub(in crate::generic_execution_v1) mod tests;
