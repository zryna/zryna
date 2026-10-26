//! Independent complete forward/commit/failure CFG and original-status retained cleanup.
use super::super::calls::{Kind, Refs, signature};
use super::{acquire, clone_frames as frames, cursor::Cursor, world::World};
use crate::generic_execution_v1::invariant;
use cranelift_codegen::ir::{
    Block, FuncRef, Function, Opcode as O, Value, condcodes::IntCC, types,
};
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
mod leaf;
mod resume;
mod rollback;
#[derive(Clone, Copy)]
pub(super) struct State {
    pub arena: Value,
    pub base: Value,
    pub context: Value,
    pub ty: Value,
    pub source: Value,
    pub destination: Value,
    pub cursor: Value,
    pub repeat: Block,
    pub failed: Block,
}
pub(super) fn again(
    reader: &mut Cursor<'_>,
    state: State,
    ty: Value,
    source: Value,
    destination: Value,
    cursor: Value,
) -> Result<(), Diagnostic> {
    if reader.jump(&[ty, source, destination, cursor])? != state.repeat {
        return Err(invariant());
    }
    Ok(())
}
pub(super) fn advance(
    reader: &mut Cursor<'_>,
    state: State,
    offset: Value,
) -> Result<(), Diagnostic> {
    let one = reader.constant(types::I32, 1)?;
    let next = reader.binary(O::Iadd, types::I32, [offset, one])?;
    again(reader, state, state.ty, state.source, state.destination, next)
}
pub(super) fn initialized(
    reader: &mut Cursor<'_>,
    refs: Refs,
    ty: Value,
    source: Value,
    destination: Value,
    context: Value,
) -> Result<(), Diagnostic> {
    let status = reader.call(refs.init, &[ty, source, destination, context], types::I32)?;
    reader.trap(status)
}
pub(super) fn failed<'f>(
    world: &mut World<'f>,
    mut reader: Cursor<'f>,
    state: State,
    status: Value,
) -> Result<Cursor<'f>, Diagnostic> {
    let (failed, ready) =
        reader.branch_args(status, &[status, state.ty, state.destination, state.cursor], &[])?;
    if failed != state.failed {
        return Err(invariant());
    }
    world.at(ready, &[])
}
pub(super) fn check(
    program: &VerifiedProgram<'_, '_>,
    function: &Function,
    refs: Refs,
    runtime: &[FuncRef],
) -> Result<(), Diagnostic> {
    if function.signature != signature(Kind::Clone)
        || !function.sized_stack_slots.is_empty()
        || !function.dynamic_stack_slots.is_empty()
    {
        return Err(invariant());
    }
    let failure_types = [types::I32, types::I32, types::I64, types::I32];
    let mut failures = function.layout.blocks().filter(|block| {
        function
            .dfg
            .block_params(*block)
            .iter()
            .map(|value| function.dfg.value_type(*value))
            .eq(failure_types)
    });
    let failed = failures.next().ok_or_else(invariant)?;
    if failures.next().is_some() {
        return Err(invariant());
    }
    let entry = function.layout.entry_block().ok_or_else(invariant)?;
    let mut world = World::new(function);
    let mut reader = world.at(entry, &[types::I32, types::I64, types::I64, types::I64])?;
    let args = function.dfg.block_params(entry);
    let arena = reader.call(refs.arena, &[args[3]], types::I64)?;
    let base = reader.load(types::I32, arena, 0)?;
    initialized(&mut reader, refs, args[0], args[1], args[2], args[3])?;
    let zero = reader.constant(types::I32, 0)?;
    let repeat = reader.jump(&[args[0], args[1], args[2], zero])?;
    let mut body = world.at(repeat, &[types::I32, types::I64, types::I64, types::I32])?;
    let values = function.dfg.block_params(repeat);
    let state = State {
        arena,
        base,
        context: args[3],
        ty: values[0],
        source: values[1],
        destination: values[2],
        cursor: values[3],
        repeat,
        failed,
    };
    let forward = body.constant(types::I32, 0)?;
    let found = body.call(
        refs.find,
        &[state.ty, state.source, state.cursor, forward, state.context],
        types::I64,
    )?;
    let absent = body.constant(types::I64, -1)?;
    let present = body.compare(IntCC::NotEqual, [found, absent])?;
    let (visit, complete) = body.branch(present)?;
    let mut visit = world.at(visit, &[])?;
    let shift = visit.constant(types::I64, 32)?;
    let high = visit.binary(O::Ushr, types::I64, [found, shift])?;
    let ty = visit.unary(O::Ireduce, types::I32, high)?;
    let offset = visit.unary(O::Ireduce, types::I32, found)?;
    leaf::check(program, &mut world, visit, refs, runtime, state, (ty, offset))?;
    let complete = world.at(complete, &[])?;
    resume::check(program, &mut world, complete, refs, state)?;
    let failure = world.at(failed, &failure_types)?;
    let values = function.dfg.block_params(failed);
    rollback::check(program, &mut world, failure, refs, state, values)?;
    world.finish()
}
pub(super) fn address(
    reader: &mut Cursor<'_>,
    root: Value,
    offset: Value,
) -> Result<Value, Diagnostic> {
    let wide = reader.unary(O::Uextend, types::I64, offset)?;
    reader.binary(O::Iadd, types::I64, [root, wide])
}
