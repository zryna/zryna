//! Exact source-certified canonical leaf roles; failed leaves never become owned prefixes.
use super::super::util::{self as u, c32, c64, cmp, load, store, trap};
use super::{Refs, State, advance, again, frames, initialized};
use crate::generic_execution_v1::{codegen, invariant, runtime};
use cranelift_codegen::ir::{FuncRef, InstBuilder, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::CloneNodeKind;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
pub(super) fn emit(
    program: &VerifiedProgram<'_, '_>,
    builder: &mut FunctionBuilder<'_>,
    refs: Refs,
    functions: &[FuncRef],
    state: State,
    leaf: Value,
    offset: Value,
) -> Result<(), Diagnostic> {
    let plan = program.containers().ok_or_else(invariant)?;
    for index in 0..program.program().type_count() {
        let ty = u32::try_from(index).map_err(codegen)?;
        let Some(node) = plan.clone_node(ty) else {
            continue;
        };
        if !matches!(
            node.kind(),
            CloneNodeKind::String
                | CloneNodeKind::VecEach
                | CloneNodeKind::SharedCount
                | CloneNodeKind::WeakCount
        ) {
            continue;
        }
        let active = builder.create_block();
        let next = builder.create_block();
        let key = c32(builder, ty);
        let selected = cmp(builder, IntCC::Equal, leaf, key);
        builder.ins().brif(selected, active, &[], next, &[]);
        builder.switch_to_block(active);
        let source = u::offset(builder, state.source, offset);
        let destination = u::offset(builder, state.destination, offset);
        if node.kind() == CloneNodeKind::VecEach {
            vector(program, builder, refs, functions, state, (ty, offset, source, destination))?;
        } else {
            terminal(
                program,
                builder,
                functions,
                state,
                (ty, offset, source, destination),
                node.kind(),
            )?;
        }
        builder.switch_to_block(next);
    }
    builder.ins().trap(cranelift_codegen::ir::TrapCode::unwrap_user(7));
    Ok(())
}
fn failed(
    builder: &mut FunctionBuilder<'_>,
    state: State,
    status: Value,
) -> cranelift_codegen::ir::Block {
    let ready = builder.create_block();
    builder.ins().brif(
        status,
        state.failed,
        &[status.into(), state.ty.into(), state.destination.into(), state.cursor.into()],
        ready,
        &[],
    );
    builder.switch_to_block(ready);
    ready
}
fn terminal(
    program: &VerifiedProgram<'_, '_>,
    builder: &mut FunctionBuilder<'_>,
    functions: &[FuncRef],
    state: State,
    target: (u32, Value, Value, Value),
    kind: CloneNodeKind,
) -> Result<(), Diagnostic> {
    let (ty, offset, source, destination) = target;
    if kind == CloneNodeKind::String {
        let status = runtime::string(
            program,
            functions,
            builder,
            state.context,
            program.string_leaf(ty, true)?,
            &[source, destination],
        )?;
        failed(builder, state, status);
        advance(builder, state, offset);
    } else {
        let pointer = load(builder, types::I64, source, 0);
        let status = runtime::control(
            program,
            functions,
            builder,
            state.context,
            program.control_clone(ty)?,
            &[pointer],
        )?;
        failed(builder, state, status);
        store(builder, destination, 0, pointer);
        advance(builder, state, offset);
    }
    Ok(())
}
fn vector(
    program: &VerifiedProgram<'_, '_>,
    builder: &mut FunctionBuilder<'_>,
    refs: Refs,
    functions: &[FuncRef],
    state: State,
    target: (u32, Value, Value, Value),
) -> Result<(), Diagnostic> {
    let (ty, offset, source, destination) = target;
    let metadata = program.containers().and_then(|p| p.element(ty)).ok_or_else(invariant)?;
    let certified = program
        .containers()
        .and_then(|p| p.clone_node(ty))
        .and_then(zryna_ir::generic_v1::execution_v1::VerifiedCloneNode::element)
        .ok_or_else(invariant)?;
    if certified.ty() != metadata.element().index() {
        return Err(invariant());
    }
    let length = load(builder, types::I64, source, 8);
    let capacity = load(builder, types::I64, source, 16);
    let max = c64(builder, zryna_ownership_runtime_abi::MAX_VEC_ELEMENTS);
    let excess = cmp(builder, IntCC::UnsignedGreaterThan, length, max);
    trap(builder, excess);
    let wrong = cmp(builder, IntCC::UnsignedGreaterThan, length, capacity);
    trap(builder, wrong);
    let element = c32(builder, metadata.element().index());
    let result = runtime::vector(
        program,
        functions,
        builder,
        state.context,
        program.vector_leaf(ty, true)?,
        &[element, length, destination],
    )?;
    failed(builder, state, result.status);
    let zero = c64(builder, 0);
    let empty = cmp(builder, IntCC::Equal, length, zero);
    let done = builder.create_block();
    let descend = builder.create_block();
    builder.ins().brif(empty, done, &[], descend, &[]);
    builder.switch_to_block(done);
    advance(builder, state, offset);
    builder.switch_to_block(descend);
    let index = c32(builder, 0);
    frames::push(
        builder,
        state.arena,
        frames::Saved {
            source: state.source,
            destination: state.destination,
            ty: state.ty,
            offset,
            index,
            vector: ty,
        },
    )?;
    let source = load(builder, types::I64, source, 0);
    let destination = load(builder, types::I64, destination, 0);
    let child = c32(builder, metadata.element().index());
    initialized(builder, refs, child, source, destination, state.context);
    let zero = c32(builder, 0);
    again(builder, state, child, source, destination, zero);
    Ok(())
}
