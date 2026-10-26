//! Independent full flat-Drop CFG, frame and exact typed canonical effect grammar.
use super::super::calls::{Kind, Refs, signature};
use super::{cursor::Cursor, frames, world::World};
use crate::generic_execution_v1::invariant;
use cranelift_codegen::ir::{
    Block, FuncRef, Function, Opcode as O, Value, condcodes::IntCC, types,
};
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
mod leaf;
mod resume;
#[derive(Clone, Copy)]
pub(super) struct Target {
    pub ty: Value,
    pub offset: Value,
}
#[derive(Clone, Copy)]
pub(super) struct State {
    pub arena: Value,
    pub ty: Value,
    pub root: Value,
    pub cursor: Value,
    pub ctx: Value,
    pub flag: Value,
    pub repeat: Block,
}
pub(super) fn again(
    reader: &mut Cursor<'_>,
    state: State,
    ty: Value,
    root: Value,
    cursor: Value,
) -> Result<(), Diagnostic> {
    if reader.jump(&[ty, root, cursor])? != state.repeat {
        return Err(invariant());
    }
    Ok(())
}
pub(super) fn address(
    reader: &mut Cursor<'_>,
    root: Value,
    offset: Value,
) -> Result<Value, Diagnostic> {
    let wide = reader.unary(O::Uextend, types::I64, offset)?;
    reader.binary(O::Iadd, types::I64, [root, wide])
}
pub(super) fn check(
    program: &VerifiedProgram<'_, '_>,
    function: &Function,
    refs: Refs,
    rt: &[FuncRef],
) -> Result<(), Diagnostic> {
    if function.signature != signature(Kind::Drop)
        || function.sized_stack_slots.len() != 1
        || !function.dynamic_stack_slots.is_empty()
    {
        return Err(invariant());
    }
    let entry = function.layout.entry_block().ok_or_else(invariant)?;
    let mut world = World::new(function);
    let mut reader = world.at(entry, &[types::I32, types::I64, types::I32, types::I64])?;
    let args = function.dfg.block_params(entry);
    let arena = reader.call(refs.arena, &[args[3]], types::I64)?;
    let base = reader.load(types::I32, arena, 0)?;
    let flag = reader.stack(4, 2)?;
    let repeat = reader.jump(&args[..3])?;
    let mut body = world.at(repeat, &[types::I32, types::I64, types::I32])?;
    let values = function.dfg.block_params(repeat);
    let state = State {
        arena,
        ty: values[0],
        root: values[1],
        cursor: values[2],
        ctx: args[3],
        flag,
        repeat,
    };
    let reverse = body.constant(types::I32, 1)?;
    let found = body.call(
        refs.find,
        &[state.ty, state.root, state.cursor, reverse, state.ctx],
        types::I64,
    )?;
    let absent = body.constant(types::I64, -1)?;
    let present = body.compare(IntCC::NotEqual, [found, absent])?;
    let (visit, restore) = body.branch(present)?;
    let mut visit = world.at(visit, &[])?;
    let shift = visit.constant(types::I64, 32)?;
    let high = visit.binary(O::Ushr, types::I64, [found, shift])?;
    let ty = visit.unary(O::Ireduce, types::I32, high)?;
    let offset = visit.unary(O::Ireduce, types::I32, found)?;
    leaf::check(program, &mut world, visit, refs, rt, state, Target { ty, offset })?;
    let mut restore = world.at(restore, &[])?;
    let used = restore.load(types::I32, arena, 0)?;
    let below = restore.compare(IntCC::UnsignedLessThan, [used, base])?;
    restore.trap(below)?;
    let finished = restore.compare(IntCC::Equal, [used, base])?;
    let (done, remaining) = restore.branch(finished)?;
    let mut done = world.at(done, &[])?;
    let zero = done.constant(types::I32, 0)?;
    done.returned(&[zero])?;
    let mut remaining = world.at(remaining, &[])?;
    let top = frames::top(&mut remaining, arena, used)?;
    resume::check(program, &mut world, remaining, refs, rt, state, top)?;
    world.finish()
}
