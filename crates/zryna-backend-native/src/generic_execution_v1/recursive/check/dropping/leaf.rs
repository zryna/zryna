//! Independent typed leaf cases, frame witnesses and lastStrong branch semantics.
use super::super::effect;
use super::{Cursor, Refs, State, Target, World, address, again, frames};
use crate::generic_execution_v1::{codegen, invariant};
use cranelift_codegen::ir::{FuncRef, Opcode as O, Value, condcodes::IntCC, types};
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::{TypeCategory, VerifiedProgram};
use zryna_ownership_runtime_abi::LogicalOperation as Role;
pub(super) fn check<'f>(
    program: &VerifiedProgram<'_, '_>,
    world: &mut World<'f>,
    mut reader: Cursor<'f>,
    refs: Refs,
    rt: &[FuncRef],
    state: State,
    target: Target,
) -> Result<(), Diagnostic> {
    for id in 0..program.program().type_count() {
        let id = u32::try_from(id).map_err(codegen)?;
        let category = program.stored_type(id).ok_or_else(invariant)?.category();
        if !matches!(
            category,
            TypeCategory::String | TypeCategory::Vec | TypeCategory::Shared | TypeCategory::Weak
        ) {
            continue;
        }
        let key = reader.constant(types::I32, i64::from(id))?;
        let matched = reader.compare(IntCC::Equal, [target.ty, key])?;
        let (yes, no) = reader.branch(matched)?;
        let mut yes = world.at(yes, &[])?;
        let operand = address(&mut yes, state.root, target.offset)?;
        match category {
            TypeCategory::String => {
                let function = effect::reference(program, rt, Role::StringRelease)?;
                let mut after = effect::region(
                    world,
                    yes,
                    effect::Call {
                        ctx: state.ctx,
                        function,
                        args: &[operand],
                        role: Role::StringRelease,
                    },
                )?;
                again(&mut after, state, state.ty, state.root, target.offset)?;
            }
            TypeCategory::Weak => {
                let control = yes.load(types::I64, operand, 0)?;
                let function = effect::reference(program, rt, Role::WeakRelease)?;
                let mut after = effect::region(
                    world,
                    yes,
                    effect::Call {
                        ctx: state.ctx,
                        function,
                        args: &[control, state.flag],
                        role: Role::WeakRelease,
                    },
                )?;
                again(&mut after, state, state.ty, state.root, target.offset)?;
            }
            TypeCategory::Shared => {
                shared(program, world, yes, rt, state, (id, target.offset, operand))?;
            }
            TypeCategory::Vec => {
                vector(program, world, yes, refs, state, (id, target.offset, operand))?;
            }
            _ => return Err(invariant()),
        }
        reader = world.at(no, &[])?;
    }
    reader.unreachable()
}
fn vector<'f>(
    program: &VerifiedProgram<'_, '_>,
    world: &mut World<'f>,
    mut reader: Cursor<'f>,
    refs: Refs,
    state: State,
    target: (u32, Value, Value),
) -> Result<(), Diagnostic> {
    let (id, offset, operand) = target;
    let metadata = program.containers().and_then(|x| x.element(id)).ok_or_else(invariant)?;
    let length = reader.load(types::I64, operand, 8)?;
    let cap = reader.load(types::I64, operand, 16)?;
    let max =
        reader.constant(types::I64, zryna_ownership_runtime_abi::MAX_VEC_ELEMENTS.cast_signed())?;
    let excess = reader.compare(IntCC::UnsignedGreaterThan, [length, max])?;
    reader.trap(excess)?;
    let bad = reader.compare(IntCC::UnsignedGreaterThan, [length, cap])?;
    reader.trap(bad)?;
    let zero = reader.constant(types::I64, 0)?;
    let empty = reader.compare(IntCC::Equal, [length, zero])?;
    let (release, descend) = reader.branch(empty)?;
    let mut release = world.at(release, &[])?;
    let key = release.constant(types::I32, i64::from(id))?;
    let status = release.call(refs.release, &[key, operand, state.ctx], types::I32)?;
    release.trap(status)?;
    again(&mut release, state, state.ty, state.root, offset)?;
    let mut descend = world.at(descend, &[])?;
    let len = descend.unary(O::Ireduce, types::I32, length)?;
    let one = descend.constant(types::I32, 1)?;
    let index = descend.binary(O::Isub, types::I32, [len, one])?;
    frames::push(
        &mut descend,
        state.arena,
        frames::Pending { leaf: id, root: state.root, ty: state.ty, offset, index, role: 0 },
    )?;
    let data = descend.load(types::I64, operand, 0)?;
    let index = descend.unary(O::Uextend, types::I64, index)?;
    let stride = descend.constant(types::I64, metadata.stride().cast_signed())?;
    let bytes = descend.binary(O::Imul, types::I64, [index, stride])?;
    let root = descend.binary(O::Iadd, types::I64, [data, bytes])?;
    let child = descend.constant(types::I32, i64::from(metadata.element().index()))?;
    let size = descend.constant(
        types::I32,
        i64::try_from(
            program.stored_type(metadata.element().index()).ok_or_else(invariant)?.size(),
        )
        .map_err(codegen)?,
    )?;
    again(&mut descend, state, child, root, size)
}
fn shared<'f>(
    program: &VerifiedProgram<'_, '_>,
    world: &mut World<'f>,
    mut reader: Cursor<'f>,
    rt: &[FuncRef],
    state: State,
    target: (u32, Value, Value),
) -> Result<(), Diagnostic> {
    let (id, offset, operand) = target;
    let metadata = program.containers().and_then(|x| x.control(id)).ok_or_else(invariant)?;
    let control = reader.load(types::I64, operand, 0)?;
    let function = effect::reference(program, rt, Role::StrongReleaseBegin)?;
    let mut after = effect::region(
        world,
        reader,
        effect::Call {
            ctx: state.ctx,
            function,
            args: &[control, state.flag],
            role: Role::StrongReleaseBegin,
        },
    )?;
    let last = after.load(types::I32, state.flag, 0)?;
    let (descend, done) = after.branch(last)?;
    let mut descend = world.at(descend, &[])?;
    let index = descend.constant(types::I32, 0)?;
    frames::push(
        &mut descend,
        state.arena,
        frames::Pending { leaf: id, root: state.root, ty: state.ty, offset, index, role: 1 },
    )?;
    let displacement = descend.constant(types::I64, metadata.payload_offset().cast_signed())?;
    let root = descend.binary(O::Iadd, types::I64, [control, displacement])?;
    let child = descend.constant(types::I32, i64::from(metadata.payload().index()))?;
    let size = descend.constant(
        types::I32,
        i64::try_from(
            program.stored_type(metadata.payload().index()).ok_or_else(invariant)?.size(),
        )
        .map_err(codegen)?,
    )?;
    again(&mut descend, state, child, root, size)?;
    again(&mut world.at(done, &[])?, state, state.ty, state.root, offset)
}
