//! Flat certified leaf Clone with exact destination-prefix commitment and original-status unwind.
use super::super::calls::Refs;
use super::{
    clone_frames as frames,
    util::{self as u, c32, c64, call, cmp, load, trap},
};
use cranelift_codegen::ir::{Block, FuncRef, InstBuilder, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
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
    builder: &mut FunctionBuilder<'_>,
    state: State,
    ty: Value,
    source: Value,
    destination: Value,
    cursor: Value,
) {
    builder
        .ins()
        .jump(state.repeat, &[ty.into(), source.into(), destination.into(), cursor.into()]);
}
pub(super) fn advance(builder: &mut FunctionBuilder<'_>, state: State, offset: Value) {
    let one = c32(builder, 1);
    let next = builder.ins().iadd(offset, one);
    again(builder, state, state.ty, state.source, state.destination, next);
}
pub(super) fn initialized(
    builder: &mut FunctionBuilder<'_>,
    refs: Refs,
    ty: Value,
    source: Value,
    destination: Value,
    context: Value,
) {
    let status = call(builder, refs.init, &[ty, source, destination, context]);
    trap(builder, status);
}
pub(super) fn emit(
    program: &VerifiedProgram<'_, '_>,
    builder: &mut FunctionBuilder<'_>,
    refs: Refs,
    runtime: &[FuncRef],
    args: &[Value],
) -> Result<(), Diagnostic> {
    let arena = call(builder, refs.arena, &[args[3]]);
    let base = load(builder, types::I32, arena, 0);
    initialized(builder, refs, args[0], args[1], args[2], args[3]);
    let repeat = u::block(builder, &[types::I32, types::I64, types::I64, types::I32]);
    let failed = u::block(builder, &[types::I32, types::I32, types::I64, types::I32]);
    let zero = c32(builder, 0);
    builder.ins().jump(repeat, &[args[0].into(), args[1].into(), args[2].into(), zero.into()]);
    builder.switch_to_block(repeat);
    let values = builder.block_params(repeat);
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
    let forward = c32(builder, 0);
    let found =
        call(builder, refs.find, &[state.ty, state.source, state.cursor, forward, state.context]);
    let absent = c64(builder, u64::MAX);
    let present = cmp(builder, IntCC::NotEqual, found, absent);
    let visit = builder.create_block();
    let complete = builder.create_block();
    builder.ins().brif(present, visit, &[], complete, &[]);
    builder.switch_to_block(visit);
    let shift = c64(builder, 32);
    let high = builder.ins().ushr(found, shift);
    let ty = builder.ins().ireduce(types::I32, high);
    let offset = builder.ins().ireduce(types::I32, found);
    leaf::emit(program, builder, refs, runtime, state, ty, offset)?;
    builder.switch_to_block(complete);
    resume::emit(program, builder, refs, state)?;
    builder.switch_to_block(failed);
    let failure = builder.block_params(failed).to_vec();
    rollback::emit(program, builder, refs, state, &failure)?;
    Ok(())
}
