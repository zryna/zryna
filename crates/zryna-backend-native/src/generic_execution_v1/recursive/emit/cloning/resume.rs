//! Commit exactly one complete element, then resume its parent or enter the next unowned slot.
use super::super::util::{self as u, c32, cmp, load, store, trap};
use super::{Refs, State, again, frames, initialized};
use crate::generic_execution_v1::{codegen, invariant};
use cranelift_codegen::ir::{InstBuilder, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::CloneNodeKind;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
pub(super) fn emit(
    program: &VerifiedProgram<'_, '_>,
    builder: &mut FunctionBuilder<'_>,
    refs: Refs,
    state: State,
) -> Result<(), Diagnostic> {
    let used = load(builder, types::I32, state.arena, 0);
    let below = cmp(builder, IntCC::UnsignedLessThan, used, state.base);
    trap(builder, below);
    let finished = cmp(builder, IntCC::Equal, used, state.base);
    let done = builder.create_block();
    let restore = builder.create_block();
    builder.ins().brif(finished, done, &[], restore, &[]);
    builder.switch_to_block(done);
    u::done(builder);
    builder.switch_to_block(restore);
    let frame = frames::top(builder, state.arena, used);
    let plan = program.containers().ok_or_else(invariant)?;
    for index in 0..program.program().type_count() {
        let ty = u32::try_from(index).map_err(codegen)?;
        if !plan.clone_node(ty).is_some_and(|n| n.kind() == CloneNodeKind::VecEach) {
            continue;
        }
        let active = builder.create_block();
        let next = builder.create_block();
        let key = c32(builder, ty);
        let matched = cmp(builder, IntCC::Equal, frame.vector, key);
        builder.ins().brif(matched, active, &[], next, &[]);
        builder.switch_to_block(active);
        vector(program, builder, refs, state, frame, ty)?;
        builder.switch_to_block(next);
    }
    builder.ins().trap(cranelift_codegen::ir::TrapCode::unwrap_user(7));
    Ok(())
}
fn vector(
    program: &VerifiedProgram<'_, '_>,
    builder: &mut FunctionBuilder<'_>,
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
    let source = u::offset(builder, frame.source, frame.offset);
    let destination = u::offset(builder, frame.destination, frame.offset);
    let one = c32(builder, 1);
    let next = builder.ins().iadd(frame.index, one);
    let wide = builder.ins().uextend(types::I64, next);
    store(builder, destination, 8, wide);
    let length = load(builder, types::I64, source, 8);
    let more = cmp(builder, IntCC::UnsignedLessThan, wide, length);
    let another = builder.create_block();
    let complete = builder.create_block();
    builder.ins().brif(more, another, &[], complete, &[]);
    builder.switch_to_block(another);
    store(builder, frame.address, 24, next);
    let source = load(builder, types::I64, source, 0);
    let destination = load(builder, types::I64, destination, 0);
    let stride = super::super::util::c64(builder, metadata.stride());
    let offset = builder.ins().imul(wide, stride);
    let source = builder.ins().iadd(source, offset);
    let destination = builder.ins().iadd(destination, offset);
    let child = c32(builder, metadata.element().index());
    initialized(builder, refs, child, source, destination, state.context);
    let zero = c32(builder, 0);
    again(builder, state, child, source, destination, zero);
    builder.switch_to_block(complete);
    store(builder, state.arena, 0, frame.previous);
    let one = c32(builder, 1);
    let cursor = builder.ins().iadd(frame.offset, one);
    again(builder, state, frame.ty, frame.source, frame.destination, cursor);
    Ok(())
}
