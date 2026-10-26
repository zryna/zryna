//! Cached parent state, exact reverse index and pop BEFORE any canonical free.
use super::super::super::calls::Refs;
use super::super::util::{self as u, c32, c64, call, cmp, load, store, trap};
use super::{State, again, frames};
use crate::generic_execution_v1::{codegen, invariant, runtime};
use cranelift_codegen::ir::{FuncRef, InstBuilder, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::{TypeCategory, VerifiedProgram};
pub(super) fn emit(
    program: &VerifiedProgram<'_, '_>,
    builder: &mut FunctionBuilder<'_>,
    refs: Refs,
    rt: &[FuncRef],
    state: State,
    frame: frames::Top,
) -> Result<(), Diagnostic> {
    let zero = c32(builder, 0);
    let vector = cmp(builder, IntCC::Equal, frame.role, zero);
    let vectors = builder.create_block();
    let controls = builder.create_block();
    builder.ins().brif(vector, vectors, &[], controls, &[]);
    builder.switch_to_block(vectors);
    dispatch(program, builder, refs, rt, state, frame, TypeCategory::Vec)?;
    builder.switch_to_block(controls);
    let one = c32(builder, 1);
    let wrong = cmp(builder, IntCC::NotEqual, frame.role, one);
    trap(builder, wrong);
    dispatch(program, builder, refs, rt, state, frame, TypeCategory::Shared)
}
fn dispatch(
    program: &VerifiedProgram<'_, '_>,
    builder: &mut FunctionBuilder<'_>,
    refs: Refs,
    rt: &[FuncRef],
    state: State,
    frame: frames::Top,
    category: TypeCategory,
) -> Result<(), Diagnostic> {
    for id in 0..program.program().type_count() {
        let id = u32::try_from(id).map_err(codegen)?;
        if program.stored_type(id).ok_or_else(invariant)?.category() != category {
            continue;
        }
        let yes = builder.create_block();
        let no = builder.create_block();
        let key = c32(builder, id);
        let matched = cmp(builder, IntCC::Equal, frame.leaf, key);
        builder.ins().brif(matched, yes, &[], no, &[]);
        builder.switch_to_block(yes);
        let address = u::offset(builder, frame.root, frame.offset);
        if category == TypeCategory::Vec {
            vector(program, builder, refs, state, frame, id, address)?;
        } else {
            let control = load(builder, types::I64, address, 0);
            store(builder, state.arena, 0, frame.previous);
            runtime::control_release(
                program,
                rt,
                builder,
                state.ctx,
                program.control_drop(id, true)?,
                &[control],
            )?;
            again(builder, state, frame.ty, frame.root, frame.offset);
        }
        builder.switch_to_block(no);
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
    id: u32,
    address: Value,
) -> Result<(), Diagnostic> {
    let zero = c32(builder, 0);
    let remains = cmp(builder, IntCC::NotEqual, frame.index, zero);
    let more = builder.create_block();
    let done = builder.create_block();
    builder.ins().brif(remains, more, &[], done, &[]);
    builder.switch_to_block(more);
    let one = c32(builder, 1);
    let index = builder.ins().isub(frame.index, one);
    store(builder, frame.address, 24, index);
    let metadata = program.containers().and_then(|c| c.element(id)).ok_or_else(invariant)?;
    let data = load(builder, types::I64, address, 0);
    let wide = builder.ins().uextend(types::I64, index);
    let stride = c64(builder, metadata.stride());
    let bytes = builder.ins().imul(wide, stride);
    let root = builder.ins().iadd(data, bytes);
    let child = c32(builder, metadata.element().index());
    let size = c32(builder, u::size(program, metadata.element().index())?);
    again(builder, state, child, root, size);
    builder.switch_to_block(done);
    store(builder, state.arena, 0, frame.previous);
    let ty = c32(builder, id);
    let status = call(builder, refs.release, &[ty, address, state.ctx]);
    trap(builder, status);
    again(builder, state, frame.ty, frame.root, frame.offset);
    Ok(())
}
