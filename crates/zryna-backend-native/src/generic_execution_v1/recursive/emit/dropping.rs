//! Flat heap traversal; only immutable by-value find recurses on the bounded inline DAG.
use super::super::calls::Refs;
use super::{
    frames,
    util::{self as u, c32, c64, call, cmp, load, trap},
};
use cranelift_codegen::ir::{
    Block, FuncRef, InstBuilder, StackSlotData, StackSlotKind, Value, condcodes::IntCC, types,
};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
mod leaf;
mod resume;
#[derive(Clone, Copy)]
pub(super) struct State {
    pub arena: Value,
    pub base: Value,
    pub ty: Value,
    pub root: Value,
    pub cursor: Value,
    pub ctx: Value,
    pub flag: Value,
    pub repeat: Block,
}
pub(super) fn again(b: &mut FunctionBuilder<'_>, s: State, ty: Value, root: Value, cursor: Value) {
    b.ins().jump(s.repeat, &[ty.into(), root.into(), cursor.into()]);
}
pub(super) fn emit(
    p: &VerifiedProgram<'_, '_>,
    b: &mut FunctionBuilder<'_>,
    r: Refs,
    runtime: &[FuncRef],
    args: &[Value],
) -> Result<(), Diagnostic> {
    let arena = call(b, r.arena, &[args[3]]);
    let base = load(b, types::I32, arena, 0);
    let slot = b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 4, 2));
    let flag = b.ins().stack_addr(types::I64, slot, 0);
    let repeat = u::block(b, &[types::I32, types::I64, types::I32]);
    let leaf = b.create_block();
    let restore = b.create_block();
    b.ins().jump(repeat, &[args[0].into(), args[1].into(), args[2].into()]);
    b.switch_to_block(repeat);
    let values = b.block_params(repeat);
    let s = State {
        arena,
        base,
        ty: values[0],
        root: values[1],
        cursor: values[2],
        ctx: args[3],
        flag,
        repeat,
    };
    let reverse = c32(b, 1);
    let result = call(b, r.find, &[s.ty, s.root, s.cursor, reverse, s.ctx]);
    let absent = c64(b, u64::MAX);
    let found = cmp(b, IntCC::NotEqual, result, absent);
    b.ins().brif(found, leaf, &[], restore, &[]);
    b.switch_to_block(leaf);
    let shift = c64(b, 32);
    let high = b.ins().ushr(result, shift);
    let leaf_ty = b.ins().ireduce(types::I32, high);
    let offset = b.ins().ireduce(types::I32, result);
    leaf::emit(p, b, r, runtime, s, leaf_ty, offset)?;
    b.switch_to_block(restore);
    let used = load(b, types::I32, s.arena, 0);
    let below = cmp(b, IntCC::UnsignedLessThan, used, s.base);
    trap(b, below);
    let finished = cmp(b, IntCC::Equal, used, s.base);
    let done = b.create_block();
    let remaining = b.create_block();
    b.ins().brif(finished, done, &[], remaining, &[]);
    b.switch_to_block(done);
    u::done(b);
    b.switch_to_block(remaining);
    let frame = frames::top(b, s.arena, used);
    resume::emit(p, b, r, runtime, s, frame)
}
