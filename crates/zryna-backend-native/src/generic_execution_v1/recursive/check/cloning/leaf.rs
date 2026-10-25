//! Certified terminal identity, unowned failed leaf and exact positive backing frame witness.
use super::{
    Cursor, Refs, State, World, acquire, address, advance, again, failed, frames, initialized,
};
use crate::generic_execution_v1::{codegen, invariant};
use cranelift_codegen::ir::{FuncRef, Value, condcodes::IntCC, types};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::CloneNodeKind;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
use zryna_ownership_runtime_abi::LogicalOperation as Role;
pub(super) fn check<'f>(
    program: &VerifiedProgram<'_, '_>,
    world: &mut World<'f>,
    mut reader: Cursor<'f>,
    refs: Refs,
    runtime: &[FuncRef],
    state: State,
    target: (Value, Value),
) -> Result<(), Diagnostic> {
    let (leaf, offset) = target;
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
        let key = reader.constant(types::I32, i64::from(ty))?;
        let selected = reader.compare(IntCC::Equal, [leaf, key])?;
        let (active, next) = reader.branch(selected)?;
        let mut active = world.at(active, &[])?;
        let source = address(&mut active, state.source, offset)?;
        let destination = address(&mut active, state.destination, offset)?;
        if node.kind() == CloneNodeKind::VecEach {
            vector(
                program,
                world,
                active,
                refs,
                runtime,
                state,
                (ty, offset, source, destination),
            )?;
        } else {
            terminal(
                program,
                world,
                active,
                runtime,
                state,
                (ty, offset, source, destination),
                node.kind(),
            )?;
        }
        reader = world.at(next, &[])?;
    }
    reader.unreachable()
}
fn terminal<'f>(
    program: &VerifiedProgram<'_, '_>,
    world: &mut World<'f>,
    mut reader: Cursor<'f>,
    runtime: &[FuncRef],
    state: State,
    target: (u32, Value, Value, Value),
    kind: CloneNodeKind,
) -> Result<(), Diagnostic> {
    let (_, offset, source, destination) = target;
    if kind == CloneNodeKind::String {
        let function = acquire::reference(program, runtime, Role::StringClone)?;
        let (reader, status) = acquire::region(
            world,
            reader,
            acquire::Call {
                context: state.context,
                function,
                args: &[source, destination],
                role: Role::StringClone,
            },
        )?;
        let mut ready = failed(world, reader, state, status)?;
        advance(&mut ready, state, offset)
    } else {
        let pointer = reader.load(types::I64, source, 0)?;
        let role =
            if kind == CloneNodeKind::SharedCount { Role::StrongClone } else { Role::WeakClone };
        let function = acquire::reference(program, runtime, role)?;
        let (reader, status) = acquire::region(
            world,
            reader,
            acquire::Call { context: state.context, function, args: &[pointer], role },
        )?;
        let mut ready = failed(world, reader, state, status)?;
        ready.store(destination, 0, pointer)?;
        advance(&mut ready, state, offset)
    }
}
fn vector<'f>(
    program: &VerifiedProgram<'_, '_>,
    world: &mut World<'f>,
    mut reader: Cursor<'f>,
    refs: Refs,
    runtime: &[FuncRef],
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
    let length = reader.load(types::I64, source, 8)?;
    let capacity = reader.load(types::I64, source, 16)?;
    let max =
        reader.constant(types::I64, zryna_ownership_runtime_abi::MAX_VEC_ELEMENTS.cast_signed())?;
    let excess = reader.compare(IntCC::UnsignedGreaterThan, [length, max])?;
    reader.trap(excess)?;
    let wrong = reader.compare(IntCC::UnsignedGreaterThan, [length, capacity])?;
    reader.trap(wrong)?;
    let element = reader.constant(types::I32, i64::from(metadata.element().index()))?;
    let function = acquire::reference(program, runtime, Role::VecAllocate)?;
    let (reader, status) = acquire::region(
        world,
        reader,
        acquire::Call {
            context: state.context,
            function,
            args: &[element, length, destination],
            role: Role::VecAllocate,
        },
    )?;
    let mut ready = failed(world, reader, state, status)?;
    let zero = ready.constant(types::I64, 0)?;
    let empty = ready.compare(IntCC::Equal, [length, zero])?;
    let (done, descend) = ready.branch(empty)?;
    advance(&mut world.at(done, &[])?, state, offset)?;
    let mut descend = world.at(descend, &[])?;
    let index = descend.constant(types::I32, 0)?;
    frames::push(
        &mut descend,
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
    let source = descend.load(types::I64, source, 0)?;
    let destination = descend.load(types::I64, destination, 0)?;
    let child = descend.constant(types::I32, i64::from(metadata.element().index()))?;
    initialized(&mut descend, refs, child, source, destination, state.context)?;
    let zero = descend.constant(types::I32, 0)?;
    again(&mut descend, state, child, source, destination, zero)
}
