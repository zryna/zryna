//! Failed leaf is excluded; pop each Clone frame before funded Drop sees that Vec.
use super::super::util::{self as u, c32, call, cmp, load, store, trap};
use super::{Refs, State, frames};
use crate::generic_execution_v1::{codegen, invariant};
use cranelift_codegen::ir::{InstBuilder, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::CloneNodeKind;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
fn drop(
    builder: &mut FunctionBuilder<'_>,
    refs: Refs,
    ty: Value,
    address: Value,
    limit: Value,
    context: Value,
) {
    let status = call(builder, refs.drop, &[ty, address, limit, context]);
    trap(builder, status);
}
pub(super) fn emit(
    program: &VerifiedProgram<'_, '_>,
    builder: &mut FunctionBuilder<'_>,
    refs: Refs,
    state: State,
    failure: &[Value],
) -> Result<(), Diagnostic> {
    drop(builder, refs, failure[1], failure[2], failure[3], state.context);
    let repeat = builder.create_block();
    builder.ins().jump(repeat, &[]);
    builder.switch_to_block(repeat);
    let used = load(builder, types::I32, state.arena, 0);
    let below = cmp(builder, IntCC::UnsignedLessThan, used, state.base);
    trap(builder, below);
    let finished = cmp(builder, IntCC::Equal, used, state.base);
    let done = builder.create_block();
    let unwind = builder.create_block();
    builder.ins().brif(finished, done, &[], unwind, &[]);
    builder.switch_to_block(done);
    builder.ins().return_(&[failure[0]]);
    builder.switch_to_block(unwind);
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
        let selected = cmp(builder, IntCC::Equal, frame.vector, key);
        builder.ins().brif(selected, active, &[], next, &[]);
        builder.switch_to_block(active);
        let destination = u::offset(builder, frame.destination, frame.offset);
        store(builder, state.arena, 0, frame.previous);
        let size = c32(builder, u::size(program, ty)?);
        drop(builder, refs, key, destination, size, state.context);
        drop(builder, refs, frame.ty, frame.destination, frame.offset, state.context);
        builder.ins().jump(repeat, &[]);
        builder.switch_to_block(next);
    }
    builder.ins().trap(cranelift_codegen::ir::TrapCode::unwrap_user(7));
    Ok(())
}
