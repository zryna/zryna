//! Complete child proof precedes prefix publication; exact bounds/index and parent restoration.
use super::{Cursor, Refs, State, World, address, again, frames, initialized};
use crate::generic_execution_v1::{codegen, invariant};
use cranelift_codegen::ir::{Opcode as O, condcodes::IntCC, types};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::CloneNodeKind;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
pub(super) fn check<'f>(
    program: &VerifiedProgram<'_, '_>,
    world: &mut World<'f>,
    mut reader: Cursor<'f>,
    refs: Refs,
    state: State,
) -> Result<(), Diagnostic> {
    let used = reader.load(types::I32, state.arena, 0)?;
    let below = reader.compare(IntCC::UnsignedLessThan, [used, state.base])?;
    reader.trap(below)?;
    let finished = reader.compare(IntCC::Equal, [used, state.base])?;
    let (done, restore) = reader.branch(finished)?;
    let mut done = world.at(done, &[])?;
    let zero = done.constant(types::I32, 0)?;
    done.returned(&[zero])?;
    let mut restore = world.at(restore, &[])?;
    let frame = frames::top(&mut restore, state.arena, used)?;
    let plan = program.containers().ok_or_else(invariant)?;
    for index in 0..program.program().type_count() {
        let ty = u32::try_from(index).map_err(codegen)?;
        if !plan.clone_node(ty).is_some_and(|n| n.kind() == CloneNodeKind::VecEach) {
            continue;
        }
        let key = restore.constant(types::I32, i64::from(ty))?;
        let matched = restore.compare(IntCC::Equal, [frame.vector, key])?;
        let (active, next) = restore.branch(matched)?;
        let active = world.at(active, &[])?;
        vector(program, world, active, refs, state, frame, ty)?;
        restore = world.at(next, &[])?;
    }
    restore.unreachable()
}
fn vector<'f>(
    program: &VerifiedProgram<'_, '_>,
    world: &mut World<'f>,
    mut reader: Cursor<'f>,
    refs: Refs,
    state: State,
    frame: frames::Top,
    ty: u32,
) -> Result<(), Diagnostic> {
    let metadata = program.containers().and_then(|p| p.element(ty)).ok_or_else(invariant)?;
    let certified = program
        .containers()
        .and_then(|p| p.clone_node(ty))
        .and_then(zryna_ir::generic_v1::execution_v1::VerifiedCloneNode::element)
        .ok_or_else(invariant)?;
    if certified.ty() != metadata.element().index() {
        return Err(invariant());
    }
    let source = address(&mut reader, frame.source, frame.offset)?;
    let destination = address(&mut reader, frame.destination, frame.offset)?;
    let one = reader.constant(types::I32, 1)?;
    let next = reader.binary(O::Iadd, types::I32, [frame.index, one])?;
    let wide = reader.unary(O::Uextend, types::I64, next)?;
    reader.store(destination, 8, wide)?;
    let length = reader.load(types::I64, source, 8)?;
    let more = reader.compare(IntCC::UnsignedLessThan, [wide, length])?;
    let (another, complete) = reader.branch(more)?;
    let mut another = world.at(another, &[])?;
    another.store(frame.address, 24, next)?;
    let source = another.load(types::I64, source, 0)?;
    let destination = another.load(types::I64, destination, 0)?;
    let stride = another.constant(types::I64, metadata.stride().cast_signed())?;
    let offset = another.binary(O::Imul, types::I64, [wide, stride])?;
    let source = another.binary(O::Iadd, types::I64, [source, offset])?;
    let destination = another.binary(O::Iadd, types::I64, [destination, offset])?;
    let child = another.constant(types::I32, i64::from(metadata.element().index()))?;
    initialized(&mut another, refs, child, source, destination, state.context)?;
    let zero = another.constant(types::I32, 0)?;
    again(&mut another, state, child, source, destination, zero)?;
    let mut complete = world.at(complete, &[])?;
    complete.store(state.arena, 0, frame.previous)?;
    let one = complete.constant(types::I32, 1)?;
    let cursor = complete.binary(O::Iadd, types::I32, [frame.offset, one])?;
    again(&mut complete, state, frame.ty, frame.source, frame.destination, cursor)
}
