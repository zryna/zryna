//! Exact typed Vec cases and closed accounted storage-effect regions.
use super::super::calls::{Kind, signature};
use super::{effect, world::World};
use crate::generic_execution_v1::{codegen, invariant};
use cranelift_codegen::ir::{FuncRef, Function, condcodes::IntCC, types};
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::{TypeCategory, VerifiedProgram};
use zryna_ownership_runtime_abi::LogicalOperation as Role;
pub(super) fn check(
    p: &VerifiedProgram<'_, '_>,
    f: &Function,
    refs: &[FuncRef],
) -> Result<(), Diagnostic> {
    if f.signature != signature(Kind::Release)
        || !f.sized_stack_slots.is_empty()
        || !f.dynamic_stack_slots.is_empty()
    {
        return Err(invariant());
    }
    let entry = f.layout.entry_block().ok_or_else(invariant)?;
    let mut w = World::new(f);
    let mut c = w.at(entry, &[types::I32, types::I64, types::I64])?;
    let args = f.dfg.block_params(entry);
    let function = effect::reference(p, refs, Role::VecReleaseStorage)?;
    for id in 0..p.program().type_count() {
        let id = u32::try_from(id).map_err(codegen)?;
        if p.stored_type(id).ok_or_else(invariant)?.category() != TypeCategory::Vec {
            continue;
        }
        let key = c.constant(types::I32, i64::from(id))?;
        let matched = c.compare(IntCC::Equal, [args[0], key])?;
        let (yes, no) = c.branch(matched)?;
        let mut yes = w.at(yes, &[])?;
        let row = p.containers().and_then(|c| c.element(id)).ok_or_else(invariant)?;
        let element = yes.constant(types::I32, i64::from(row.element().index()))?;
        let mut after = effect::region(
            &mut w,
            yes,
            effect::Call {
                ctx: args[2],
                function,
                args: &[element, args[1]],
                role: Role::VecReleaseStorage,
            },
        )?;
        let zero = after.constant(types::I32, 0)?;
        after.returned(&[zero])?;
        c = w.at(no, &[])?;
    }
    c.unreachable()?;
    w.finish()
}
