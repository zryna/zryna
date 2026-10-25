//! Independent closed actual-SSA recipes; emitter bodies are never the proof input.
use super::calls::{Kind, Refs};
use cranelift_codegen::ir::{FuncRef, Function};
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
mod acquire;
mod arena;
pub(in crate::generic_execution_v1) mod clone_forward;
mod clone_frames;
mod cloning;
mod cursor;
mod dropping;
mod effect;
mod find;
pub(in crate::generic_execution_v1) mod forward;
mod frames;
mod initialize;
mod lease;
mod release;
mod world;
pub(in crate::generic_execution_v1) mod wrapper;
pub(super) fn function(
    p: &VerifiedProgram<'_, '_>,
    f: &Function,
    kind: Kind,
    refs: Refs,
    runtime: &[FuncRef],
) -> Result<(), Diagnostic> {
    if !matches!(kind, Kind::Arena) {
        let entry = f.layout.entry_block().ok_or_else(crate::generic_execution_v1::invariant)?;
        let ctx =
            *f.dfg.block_params(entry).last().ok_or_else(crate::generic_execution_v1::invariant)?;
        crate::generic_execution_v1::accounting::audit_function(p, f, ctx, runtime)?;
    }
    match kind {
        Kind::Arena => arena::check(f, refs),
        Kind::Find => find::check(p, f, refs),
        Kind::Drop => dropping::check(p, f, refs, runtime),
        Kind::Release => release::check(p, f, runtime),
        Kind::Init => initialize::check(p, f, refs),
        Kind::Clone => cloning::check(p, f, refs, runtime),
    }
}
