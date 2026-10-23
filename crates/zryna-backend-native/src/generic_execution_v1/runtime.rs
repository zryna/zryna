//! Exact native declarations from this same program's successor runtime issuer.
use super::{codegen, invariant};
use cranelift_codegen::{
    ir::{AbiParam, FuncRef, InstBuilder, Signature, Value, types},
    isa::CallConv,
};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::{FuncId, Linkage, Module};
use cranelift_object::ObjectModule;
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::{ContainerPlan, VerifiedProgram};
use zryna_ownership_runtime_abi::{LogicalOperation, raw::NativeCarrier};
pub(super) const NAMES: [&str; 3] = ["stringFromUtf8Copy", "stringClone", "stringRelease"];
fn operations(p: &VerifiedProgram<'_, '_>) -> Vec<&'static str> {
    NAMES
        .into_iter()
        .chain(p.containers().into_iter().flat_map(ContainerPlan::operations).map(|op| op.name()))
        .chain(p.uses_string_concat().then_some("stringConcat"))
        .collect()
}
pub(super) fn names<'a>(p: &'a VerifiedProgram<'_, '_>) -> Result<Vec<&'a str>, Diagnostic> {
    operations(p)
        .into_iter()
        .map(|op| {
            p.program()
                .runtime()
                .declarations()
                .native_linux_x86_64
                .iter()
                .find(|d| d.operation == op)
                .map(|d| d.symbol.as_str())
                .ok_or_else(invariant)
        })
        .collect()
}
pub(super) fn declare(
    object: &mut ObjectModule,
    p: &VerifiedProgram<'_, '_>,
) -> Result<Vec<FuncId>, Diagnostic> {
    operations(p)
        .into_iter()
        .map(|op| {
            let declaration = p
                .program()
                .runtime()
                .declarations()
                .native_linux_x86_64
                .iter()
                .find(|d| d.operation == op)
                .ok_or_else(invariant)?;
            let mut signature = Signature::new(CallConv::SystemV);
            for carrier in &declaration.parameters {
                signature.params.push(AbiParam::new(if *carrier == NativeCarrier::U32 {
                    types::I32
                } else {
                    types::I64
                }));
            }
            signature.returns.push(AbiParam::new(types::I32));
            object
                .declare_function(&declaration.symbol, Linkage::Import, &signature)
                .map_err(codegen)
        })
        .collect()
}
pub(super) fn call(
    p: &VerifiedProgram<'_, '_>,
    refs: &[FuncRef],
    b: &mut FunctionBuilder<'_>,
    op: LogicalOperation,
    args: &[Value],
) -> Result<Value, Diagnostic> {
    let index = operations(p).iter().position(|name| *name == op.name()).ok_or_else(invariant)?;
    let instruction = b.ins().call(*refs.get(index).ok_or_else(invariant)?, args);
    Ok(b.inst_results(instruction)[0])
}
