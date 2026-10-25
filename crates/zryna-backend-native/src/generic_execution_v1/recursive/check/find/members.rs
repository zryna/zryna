//! Independent exact tag/field branch recipes with complete block/operator coverage.
use super::{Cursor, Input, Refs, World, child, none};
use crate::generic_execution_v1::{codegen, invariant};
use cranelift_codegen::ir::{condcodes::IntCC, types};
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::{TypeCategory, VerifiedProgram};
pub(super) fn check<'f>(
    program: &VerifiedProgram<'_, '_>,
    world: &mut World<'f>,
    mut reader: Cursor<'f>,
    refs: Refs,
    id: u32,
    input: Input,
) -> Result<(), Diagnostic> {
    let view = program.stored_type(id).ok_or_else(invariant)?;
    if view.category() == TypeCategory::Enum {
        let tag = reader.load(types::I32, input.root, 0)?;
        for (ordinal, payload) in view.variants() {
            let key = reader.constant(types::I32, i64::from(ordinal))?;
            let matched = reader.compare(IntCC::Equal, [tag, key])?;
            let (yes, next) = reader.branch(matched)?;
            let mut arm = world.at(yes, &[])?;
            if let Some(payload) = payload {
                let missing = child(
                    world,
                    arm,
                    refs,
                    payload.index(),
                    u32::try_from(view.payload().ok_or_else(invariant)?.0).map_err(codegen)?,
                    input,
                )?;
                arm = world.at(missing, &[])?;
            }
            none(&mut arm)?;
            reader = world.at(next, &[])?;
        }
        return reader.unreachable();
    }
    let mut fields = Vec::<(u32, u32)>::new();
    fields.try_reserve_exact(view.fields().len()).map_err(codegen)?;
    for (_, ty, offset) in view.fields() {
        fields.push((ty.index(), u32::try_from(offset).map_err(codegen)?));
    }
    let zero = reader.constant(types::I32, 0)?;
    let backwards = reader.compare(IntCC::NotEqual, [input.reverse, zero])?;
    let (reverse, forward) = reader.branch(backwards)?;
    reader = world.at(reverse, &[])?;
    for &(ty, offset) in fields.iter().rev() {
        let next = child(world, reader, refs, ty, offset, input)?;
        reader = world.at(next, &[])?;
    }
    none(&mut reader)?;
    reader = world.at(forward, &[])?;
    for (ty, offset) in fields {
        let next = child(world, reader, refs, ty, offset, input)?;
        reader = world.at(next, &[])?;
    }
    none(&mut reader)
}
