//! Unchanged scalar ABI with private physical records and status-before-output checks.
use cranelift_codegen::{
    Context,
    ir::{
        AbiParam, FuncRef, InstBuilder, MemFlagsData, Signature, StackSlotData, StackSlotKind,
        TrapCode, condcodes::IntCC, types,
    },
    isa::CallConv,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use zryna_abi::{ScalarType, VerifiedScalarExport};
pub(super) fn signature(export: VerifiedScalarExport<'_>) -> Signature {
    let mut s = Signature::new(CallConv::SystemV);
    for _ in export.parameters() {
        s.params.push(AbiParam::new(types::I32));
    }
    s.returns.push(AbiParam::new(types::I32));
    s
}
pub(super) fn build(
    export: VerifiedScalarExport<'_>,
    callee: FuncRef,
    context: &mut Context,
    bc: &mut FunctionBuilderContext,
    config: cranelift_codegen::isa::TargetFrontendConfig,
    tls: Option<FuncRef>,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    let mut b = FunctionBuilder::new(&mut context.func, bc);
    let block = b.create_block();
    b.append_block_params_for_function_params(block);
    b.switch_to_block(block);
    b.seal_block(block);
    let values = b.block_params(block).to_vec();
    let output = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
    let output_address = b.ins().stack_addr(types::I64, output, 0);
    let mut arguments = vec![output_address];
    for (value, ty) in values.into_iter().zip(export.parameters()) {
        if *ty == ScalarType::Bool {
            guard(&mut b, value);
        }
        let slot = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
        let address = b.ins().stack_addr(types::I64, slot, 0);
        let zero = b.ins().iconst(types::I64, 0);
        b.ins().store(MemFlagsData::new(), zero, address, 0);
        let value = if *ty == ScalarType::Bool { b.ins().ireduce(types::I8, value) } else { value };
        b.ins().store(MemFlagsData::new(), value, address, 0);
        arguments.push(address);
    }
    let invocation =
        b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 32, 3));
    let invocation_address = b.ins().stack_addr(types::I64, invocation, 0);
    let zero = b.ins().iconst(types::I64, 0);
    for offset in [0, 8, 16, 24] {
        b.ins().store(MemFlagsData::new(), zero, invocation_address, offset);
    }
    let lease =
        tls.map(|getter| super::recursive::lifecycle::begin(&mut b, getter, invocation_address));
    arguments.push(invocation_address);
    let call = b.ins().call(callee, &arguments);
    let status = b.inst_results(call)[0];
    if let Some(base) = lease {
        super::recursive::lifecycle::completed(&mut b, base, invocation_address);
    }
    super::accounting::finish(&mut b, invocation_address);
    if let Some(base) = lease {
        super::recursive::lifecycle::end(&mut b, base);
    }
    b.ins().trapnz(status, TrapCode::unwrap_user(7));
    let result = if export.result() == ScalarType::Bool {
        let value = b.ins().load(types::I8, MemFlagsData::new(), output_address, 0);
        let result = b.ins().uextend(types::I32, value);
        guard(&mut b, result);
        result
    } else {
        b.ins().load(types::I32, MemFlagsData::new(), output_address, 0)
    };
    b.ins().return_(&[result]);
    if let Some(getter) = tls {
        super::recursive::check::wrapper::check(b.func, export, callee, getter)?;
    }
    b.finalize(config);
    Ok(())
}
fn guard(b: &mut FunctionBuilder<'_>, value: cranelift_codegen::ir::Value) {
    let invalid = b.ins().icmp_imm_u(IntCC::UnsignedGreaterThan, value, 1);
    b.ins().trapnz(invalid, TrapCode::unwrap_user(7));
}
