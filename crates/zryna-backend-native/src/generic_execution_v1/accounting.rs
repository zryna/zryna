//! NEW invocation counters; canonical C signatures and nested factoring stay unchanged.
use super::invariant;
use cranelift_codegen::ir::{
    FuncRef, InstBuilder, MemFlagsData, TrapCode, Value, condcodes::IntCC, types,
};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_ownership_runtime_abi::{
    LogicalOperation as O, MAX_ALLOCATION_OPERATIONS as A, MAX_LIVE_ALLOCATIONS as L,
    MAX_STATUS_TRANSITIONS as S, MAX_STRING_BYTES,
};
mod acquire;
mod audit;
pub(super) mod control;
mod release;
mod state;
pub(super) mod vector;
pub(in crate::generic_execution_v1) use acquire::string;
pub(in crate::generic_execution_v1) use release::release;
use state::{checked, constant, status, store};
#[derive(Clone, Copy)]
struct State {
    a: Value,
    l: Value,
    s: Value,
    h: Value,
}
pub(super) fn finish(b: &mut FunctionBuilder<'_>, context: Value) {
    let q = checked(b, context);
    let left = b.ins().bor(q.l, q.h);
    b.ins().trapnz(left, TrapCode::unwrap_user(7));
}
pub(super) fn audit_function(
    p: &zryna_native_mir::generic_execution_v1::VerifiedProgram<'_, '_>,
    f: &cranelift_codegen::ir::Function,
    context: Value,
    runtime: &[FuncRef],
) -> Result<(), Diagnostic> {
    audit::function(p, f, context, runtime)
}
