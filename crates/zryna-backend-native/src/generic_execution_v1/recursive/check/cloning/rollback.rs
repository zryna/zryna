//! Complete partial-prefix unwind; original status survives every mandatory Drop.
use super::{Cursor, Refs, State, World, address, frames};
use crate::generic_execution_v1::{codegen, invariant};
use cranelift_codegen::ir::{Value, condcodes::IntCC, types};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::CloneNodeKind;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
fn drop(
    reader: &mut Cursor<'_>,
    refs: Refs,
    ty: Value,
    root: Value,
    limit: Value,
    context: Value,
) -> Result<(), Diagnostic> {
    let status = reader.call(refs.drop, &[ty, root, limit, context], types::I32)?;
    reader.trap(status)
}
pub(super) fn check<'f>(
    program: &VerifiedProgram<'_, '_>,
    world: &mut World<'f>,
    mut reader: Cursor<'f>,
    refs: Refs,
    state: State,
    failure: &[Value],
) -> Result<(), Diagnostic> {
    drop(&mut reader, refs, failure[1], failure[2], failure[3], state.context)?;
    let repeat = reader.jump(&[])?;
    let mut header = world.at(repeat, &[])?;
    let used = header.load(types::I32, state.arena, 0)?;
    let below = header.compare(IntCC::UnsignedLessThan, [used, state.base])?;
    header.trap(below)?;
    let finished = header.compare(IntCC::Equal, [used, state.base])?;
    let (done, unwind) = header.branch(finished)?;
    let mut done = world.at(done, &[])?;
    done.returned(&[failure[0]])?;
    let mut unwind = world.at(unwind, &[])?;
    let frame = frames::top(&mut unwind, state.arena, used)?;
    let plan = program.containers().ok_or_else(invariant)?;
    for index in 0..program.program().type_count() {
        let ty = u32::try_from(index).map_err(codegen)?;
        if !plan.clone_node(ty).is_some_and(|n| n.kind() == CloneNodeKind::VecEach) {
            continue;
        }
        let key = unwind.constant(types::I32, i64::from(ty))?;
        let selected = unwind.compare(IntCC::Equal, [frame.vector, key])?;
        let (active, next) = unwind.branch(selected)?;
        let mut active = world.at(active, &[])?;
        let destination = address(&mut active, frame.destination, frame.offset)?;
        active.store(state.arena, 0, frame.previous)?;
        let size = u32::try_from(program.stored_type(ty).ok_or_else(invariant)?.size())
            .map_err(codegen)?;
        let size = active.constant(types::I32, i64::from(size))?;
        drop(&mut active, refs, key, destination, size, state.context)?;
        drop(&mut active, refs, frame.ty, frame.destination, frame.offset, state.context)?;
        if active.jump(&[])? != repeat {
            return Err(invariant());
        }
        unwind = world.at(next, &[])?;
    }
    unwind.unreachable()
}
