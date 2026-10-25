//! Exact certified per-type forward; no recursive old helper or rollback call can survive.
use super::cursor::Cursor;
use crate::generic_execution_v1::invariant;
use cranelift_codegen::ir::{FuncRef, Function, types};
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
pub(in crate::generic_execution_v1) fn check(
    program: &VerifiedProgram<'_, '_>,
    function: &Function,
    ty: u32,
    callee: FuncRef,
) -> Result<(), Diagnostic> {
    if program.containers().and_then(|p| p.clone_node(ty)).is_none()
        || !function.sized_stack_slots.is_empty()
        || !function.dynamic_stack_slots.is_empty()
    {
        return Err(invariant());
    }
    let entry = function.layout.entry_block().ok_or_else(invariant)?;
    if function.layout.next_block(entry).is_some() {
        return Err(invariant());
    }
    let args = function.dfg.block_params(entry);
    if args.len() != 3 || args.iter().any(|v| function.dfg.value_type(*v) != types::I64) {
        return Err(invariant());
    }
    let mut reader = Cursor::new(function, entry);
    let key = reader.constant(types::I32, i64::from(ty))?;
    let result = reader.call(callee, &[key, args[0], args[1], args[2]], types::I32)?;
    reader.returned(&[result])
}
