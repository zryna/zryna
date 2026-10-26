//! Finite emitters for the independently matched private traversal protocol.
use super::calls::{Kind, Refs};
use cranelift_codegen::ir::{FuncRef, Value};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
mod arena;
mod clone_frames;
mod cloning;
mod dropping;
mod find;
mod frames;
mod initialize;
mod release;
pub(super) mod util;
pub(super) fn build(
    p: &VerifiedProgram<'_, '_>,
    b: &mut FunctionBuilder<'_>,
    kind: Kind,
    refs: Refs,
    runtime: &[FuncRef],
    args: &[Value],
) -> Result<(), Diagnostic> {
    match kind {
        Kind::Arena => arena::emit(b, refs, args[0]),
        Kind::Find => find::emit(p, b, refs, args),
        Kind::Drop => dropping::emit(p, b, refs, runtime, args),
        Kind::Release => release::emit(p, b, refs, runtime, args),
        Kind::Init => initialize::emit(p, b, refs, args),
        Kind::Clone => cloning::emit(p, b, refs, runtime, args),
    }
}
