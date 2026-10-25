//! Exact same-owner Vec metadata and unchanged funded canonical storage release.
use super::super::calls::Refs;
use super::util::{c32, cmp, done};
use crate::generic_execution_v1::{codegen, invariant, runtime};
use cranelift_codegen::ir::{InstBuilder, Value, condcodes::IntCC};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::{TypeCategory, VerifiedProgram};
pub(super) fn emit(
    p: &VerifiedProgram<'_, '_>,
    b: &mut FunctionBuilder<'_>,
    _r: Refs,
    runtime: &[cranelift_codegen::ir::FuncRef],
    args: &[Value],
) -> Result<(), Diagnostic> {
    for id in 0..p.program().type_count() {
        let id = u32::try_from(id).map_err(codegen)?;
        if p.stored_type(id).ok_or_else(invariant)?.category() != TypeCategory::Vec {
            continue;
        }
        let yes = b.create_block();
        let no = b.create_block();
        let key = c32(b, id);
        let matched = cmp(b, IntCC::Equal, args[0], key);
        b.ins().brif(matched, yes, &[], no, &[]);
        b.switch_to_block(yes);
        let metadata = p.containers().and_then(|c| c.element(id)).ok_or_else(invariant)?;
        let element = c32(b, metadata.element().index());
        runtime::vector_release(
            p,
            runtime,
            b,
            args[2],
            p.vector_leaf(id, false)?,
            &[element, args[1]],
        )?;
        done(b);
        b.switch_to_block(no);
    }
    b.ins().trap(cranelift_codegen::ir::TrapCode::unwrap_user(7));
    Ok(())
}
