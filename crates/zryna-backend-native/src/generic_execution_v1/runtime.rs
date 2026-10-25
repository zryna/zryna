//! Exact native declarations from this same program's successor runtime issuer.
use super::{codegen, invariant};
use cranelift_codegen::{
    ir::{AbiParam, FuncRef, Signature, Value, types},
    isa::CallConv,
};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::{FuncId, Linkage, Module};
use cranelift_object::ObjectModule;
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::{ContainerPlan, VerifiedProgram};
use zryna_ownership_runtime_abi::{LogicalOperation, raw::NativeCarrier};
mod control;
pub(super) use control::{control, control_operation, control_release};
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

pub(super) fn string(
    p: &VerifiedProgram<'_, '_>,
    refs: &[FuncRef],
    b: &mut FunctionBuilder<'_>,
    invocation: Value,
    effect: zryna_native_mir::generic_execution_v1::StringEffect<'_, '_>,
    args: &[Value],
) -> Result<Value, Diagnostic> {
    if !effect.belongs_to(p.program()) {
        return Err(invariant());
    }
    let op = effect.operation();
    let index = match op {
        LogicalOperation::StringFromUtf8Copy => 0,
        LogicalOperation::StringClone => 1,
        LogicalOperation::StringConcat if p.uses_string_concat() => {
            refs.len().checked_sub(1).ok_or_else(invariant)?
        }
        _ => return Err(invariant()),
    };
    super::accounting::string(b, invocation, *refs.get(index).ok_or_else(invariant)?, effect, args)
}
pub(super) fn string_release(
    p: &VerifiedProgram<'_, '_>,
    refs: &[FuncRef],
    b: &mut FunctionBuilder<'_>,
    invocation: Value,
    address: Value,
) -> Result<(), Diagnostic> {
    if p.program().runtime().declarations().native_linux_x86_64.is_empty() {
        return Err(invariant());
    }
    super::accounting::release(b, invocation, *refs.get(2).ok_or_else(invariant)?, address)
}

pub(super) fn string_operation(
    p: &VerifiedProgram<'_, '_>,
    refs: &[FuncRef],
    function: FuncRef,
) -> Option<LogicalOperation> {
    for (index, op) in [
        (0, LogicalOperation::StringFromUtf8Copy),
        (1, LogicalOperation::StringClone),
        (2, LogicalOperation::StringRelease),
    ] {
        if refs.get(index) == Some(&function) {
            return Some(op);
        }
    }
    if p.uses_string_concat() && refs.last() == Some(&function) {
        return Some(LogicalOperation::StringConcat);
    }
    None
}

/// All Vec guards retain the same C FuncRef/operands; only source status gets a denial join.
pub(super) fn vector(
    p: &VerifiedProgram<'_, '_>,
    refs: &[FuncRef],
    b: &mut FunctionBuilder<'_>,
    invocation: Value,
    effect: zryna_native_mir::generic_execution_v1::VectorEffect<'_, '_>,
    args: &[Value],
) -> Result<super::accounting::vector::Call, Diagnostic> {
    if !effect.belongs_to(p.program()) {
        return Err(invariant());
    }
    let index = NAMES.len()
        + p.containers()
            .ok_or_else(invariant)?
            .operations()
            .iter()
            .position(|op| *op == effect.operation())
            .ok_or_else(invariant)?;
    super::accounting::vector::call(
        b,
        invocation,
        *refs.get(index).ok_or_else(invariant)?,
        effect,
        args,
    )
}
pub(super) fn vector_release(
    p: &VerifiedProgram<'_, '_>,
    refs: &[FuncRef],
    b: &mut FunctionBuilder<'_>,
    invocation: Value,
    effect: zryna_native_mir::generic_execution_v1::VectorEffect<'_, '_>,
    args: &[Value],
) -> Result<(), Diagnostic> {
    if !effect.belongs_to(p.program()) {
        return Err(invariant());
    }
    let index = NAMES.len()
        + p.containers()
            .ok_or_else(invariant)?
            .operations()
            .iter()
            .position(|op| *op == effect.operation())
            .ok_or_else(invariant)?;
    super::accounting::vector::release(
        b,
        invocation,
        *refs.get(index).ok_or_else(invariant)?,
        effect,
        args,
    )
}
pub(super) fn vector_operation(
    p: &VerifiedProgram<'_, '_>,
    refs: &[FuncRef],
    function: FuncRef,
) -> Option<LogicalOperation> {
    use LogicalOperation as O;
    let plan = p.containers()?;
    for op in [O::VecAllocate, O::VecReserve, O::VecReleaseStorage] {
        if let Some(index) = plan.operations().iter().position(|candidate| *candidate == op)
            && refs.get(NAMES.len() + index) == Some(&function)
        {
            return Some(op);
        }
    }
    None
}
