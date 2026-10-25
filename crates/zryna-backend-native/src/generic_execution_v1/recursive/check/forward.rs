//! Complete forwarding recipe; no old recursive heap edge can remain in this helper.
use super::cursor::Cursor;
use crate::generic_execution_v1::{codegen, invariant};
use cranelift_codegen::ir::{FuncRef, Function, types};
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
pub(in crate::generic_execution_v1) fn check(
    p: &VerifiedProgram<'_, '_>,
    f: &Function,
    ty: u32,
    drop: FuncRef,
) -> Result<(), Diagnostic> {
    if !f.sized_stack_slots.is_empty() || !f.dynamic_stack_slots.is_empty() {
        return Err(invariant());
    }
    let entry = f.layout.entry_block().ok_or_else(invariant)?;
    if f.layout.next_block(entry).is_some() {
        return Err(invariant());
    }
    let args = f.dfg.block_params(entry);
    if args.len() != 2 || args.iter().any(|v| f.dfg.value_type(*v) != types::I64) {
        return Err(invariant());
    }
    let view = p.stored_type(ty).ok_or_else(invariant)?;
    let mut c = Cursor::new(f, entry);
    let key = c.constant(types::I32, i64::from(ty))?;
    let size = c.constant(types::I32, i64::from(u32::try_from(view.size()).map_err(codegen)?))?;
    let status = c.call(drop, &[key, args[0], size, args[1]], types::I32)?;
    c.returned(&[status])
}
