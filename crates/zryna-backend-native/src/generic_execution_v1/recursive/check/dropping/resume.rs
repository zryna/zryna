//! Independent restoration, typed reverse progress and pop-before-free canonical closure.
use super::super::effect;
use super::{Cursor, Refs, State, World, address, again, frames};
use crate::generic_execution_v1::{codegen, invariant};
use cranelift_codegen::ir::{FuncRef, Opcode as O, Value, condcodes::IntCC, types};
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::{TypeCategory, VerifiedProgram};
use zryna_ownership_runtime_abi::LogicalOperation as Role;
pub(super) fn check<'frame>(
    program: &VerifiedProgram<'_, '_>,
    world: &mut World<'frame>,
    mut reader: Cursor<'frame>,
    refs: Refs,
    rt: &[FuncRef],
    state: State,
    frame: frames::Top,
) -> Result<(), Diagnostic> {
    let zero = reader.constant(types::I32, 0)?;
    let vector = reader.compare(IntCC::Equal, [frame.role, zero])?;
    let (vectors, controls) = reader.branch(vector)?;
    let vectors = world.at(vectors, &[])?;
    dispatch(program, world, vectors, refs, rt, state, (frame, TypeCategory::Vec))?;
    let mut controls = world.at(controls, &[])?;
    let one = controls.constant(types::I32, 1)?;
    let wrong = controls.compare(IntCC::NotEqual, [frame.role, one])?;
    controls.trap(wrong)?;
    dispatch(program, world, controls, refs, rt, state, (frame, TypeCategory::Shared))
}
fn dispatch<'frame>(
    program: &VerifiedProgram<'_, '_>,
    world: &mut World<'frame>,
    mut reader: Cursor<'frame>,
    refs: Refs,
    rt: &[FuncRef],
    state: State,
    target: (frames::Top, TypeCategory),
) -> Result<(), Diagnostic> {
    let (frame, category) = target;
    for id in 0..program.program().type_count() {
        let id = u32::try_from(id).map_err(codegen)?;
        if program.stored_type(id).ok_or_else(invariant)?.category() != category {
            continue;
        }
        let key = reader.constant(types::I32, i64::from(id))?;
        let matched = reader.compare(IntCC::Equal, [frame.leaf, key])?;
        let (yes, no) = reader.branch(matched)?;
        let mut yes = world.at(yes, &[])?;
        let operand = address(&mut yes, frame.root, frame.offset)?;
        if category == TypeCategory::Vec {
            vector(program, world, yes, refs, state, (frame, id, operand))?;
        } else {
            let control = yes.load(types::I64, operand, 0)?;
            yes.store(state.arena, 0, frame.previous)?;
            let function = effect::reference(program, rt, Role::StrongReleaseFinish)?;
            let mut after = effect::region(
                world,
                yes,
                effect::Call {
                    ctx: state.ctx,
                    function,
                    args: &[control],
                    role: Role::StrongReleaseFinish,
                },
            )?;
            again(&mut after, state, frame.ty, frame.root, frame.offset)?;
        }
        reader = world.at(no, &[])?;
    }
    reader.unreachable()
}
fn vector<'frame>(
    program: &VerifiedProgram<'_, '_>,
    world: &mut World<'frame>,
    mut reader: Cursor<'frame>,
    refs: Refs,
    state: State,
    target: (frames::Top, u32, Value),
) -> Result<(), Diagnostic> {
    let (frame, id, operand) = target;
    let zero = reader.constant(types::I32, 0)?;
    let remains = reader.compare(IntCC::NotEqual, [frame.index, zero])?;
    let (more, done) = reader.branch(remains)?;
    let mut more = world.at(more, &[])?;
    let one = more.constant(types::I32, 1)?;
    let index = more.binary(O::Isub, types::I32, [frame.index, one])?;
    more.store(frame.address, 24, index)?;
    let metadata = program.containers().and_then(|x| x.element(id)).ok_or_else(invariant)?;
    let data = more.load(types::I64, operand, 0)?;
    let wide = more.unary(O::Uextend, types::I64, index)?;
    let stride = more.constant(types::I64, metadata.stride().cast_signed())?;
    let bytes = more.binary(O::Imul, types::I64, [wide, stride])?;
    let root = more.binary(O::Iadd, types::I64, [data, bytes])?;
    let child = more.constant(types::I32, i64::from(metadata.element().index()))?;
    let size = more.constant(
        types::I32,
        i64::try_from(
            program.stored_type(metadata.element().index()).ok_or_else(invariant)?.size(),
        )
        .map_err(codegen)?,
    )?;
    again(&mut more, state, child, root, size)?;
    let mut done = world.at(done, &[])?;
    done.store(state.arena, 0, frame.previous)?;
    let key = done.constant(types::I32, i64::from(id))?;
    let status = done.call(refs.release, &[key, operand, state.ctx], types::I32)?;
    done.trap(status)?;
    again(&mut done, state, frame.ty, frame.root, frame.offset)
}
