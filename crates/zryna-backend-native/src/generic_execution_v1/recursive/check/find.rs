//! Independent closed finder grammar; source layouts supply constants, never emitter bodies.
use super::super::calls::{Kind, Refs, signature};
use super::{cursor::Cursor, world::World};
use crate::generic_execution_v1::{codegen, invariant};
use cranelift_codegen::ir::{Block, Function, Opcode as O, Value, condcodes::IntCC, types};
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::{TypeCategory, VerifiedProgram};
mod array;
mod members;
#[derive(Clone, Copy)]
pub(super) struct Input {
    pub root: Value,
    pub cursor: Value,
    pub reverse: Value,
    pub ctx: Value,
}
pub(super) fn check(
    program: &VerifiedProgram<'_, '_>,
    function: &Function,
    refs: Refs,
) -> Result<(), Diagnostic> {
    if function.signature != signature(Kind::Find)
        || !function.sized_stack_slots.is_empty()
        || !function.dynamic_stack_slots.is_empty()
    {
        return Err(invariant());
    }
    let entry = function.layout.entry_block().ok_or_else(invariant)?;
    let mut world = World::new(function);
    let mut reader =
        world.at(entry, &[types::I32, types::I64, types::I32, types::I32, types::I64])?;
    let args = function.dfg.block_params(entry);
    let input = Input { root: args[1], cursor: args[2], reverse: args[3], ctx: args[4] };
    for id in 0..program.program().type_count() {
        let id = u32::try_from(id).map_err(codegen)?;
        let key = reader.constant(types::I32, i64::from(id))?;
        let selected = reader.compare(IntCC::Equal, [args[0], key])?;
        let (yes, next) = reader.branch(selected)?;
        let mut active = world.at(yes, &[])?;
        let view = program.stored_type(id).ok_or_else(invariant)?;
        if view.size() == 0 {
            none(&mut active)?;
        } else {
            match view.category() {
                TypeCategory::Bool | TypeCategory::I32 => none(&mut active)?,
                TypeCategory::String
                | TypeCategory::Vec
                | TypeCategory::Shared
                | TypeCategory::Weak => {
                    terminal(&mut world, active, id, input)?;
                }
                TypeCategory::FixedArray => {
                    array::check(program, &mut world, active, refs, id, input)?;
                }
                TypeCategory::Struct | TypeCategory::Enum => {
                    members::check(program, &mut world, active, refs, id, input)?;
                }
            }
        }
        reader = world.at(next, &[])?;
    }
    reader.unreachable()?;
    world.finish()
}
pub(super) fn none(reader: &mut Cursor<'_>) -> Result<(), Diagnostic> {
    let n = reader.constant(types::I64, -1)?;
    reader.returned(&[n])
}
fn terminal<'function>(
    world: &mut World<'function>,
    mut reader: Cursor<'function>,
    id: u32,
    input: Input,
) -> Result<(), Diagnostic> {
    let zero = reader.constant(types::I32, 0)?;
    let reversing = reader.compare(IntCC::NotEqual, [input.reverse, zero])?;
    let backward = reader.compare(IntCC::NotEqual, [input.cursor, zero])?;
    let forward = reader.compare(IntCC::Equal, [input.cursor, zero])?;
    let found = reader.ternary(O::Select, types::I8, [reversing, backward, forward])?;
    let (yes, no) = reader.branch(found)?;
    let mut present = world.at(yes, &[])?;
    let result = present.constant(types::I64, (u64::from(id) << 32).cast_signed())?;
    present.returned(&[result])?;
    none(&mut world.at(no, &[])?)
}
pub(super) fn child<'function>(
    world: &mut World<'function>,
    mut reader: Cursor<'function>,
    refs: Refs,
    ty: u32,
    offset: u32,
    input: Input,
) -> Result<Block, Diagnostic> {
    let off = reader.constant(types::I32, i64::from(offset))?;
    let width = reader.unary(O::Uextend, types::I64, off)?;
    let root = reader.binary(O::Iadd, types::I64, [input.root, width])?;
    let zero = reader.constant(types::I32, 0)?;
    let before = reader.compare(IntCC::UnsignedLessThan, [input.cursor, off])?;
    let relative = reader.binary(O::Isub, types::I32, [input.cursor, off])?;
    let cursor = reader.ternary(O::Select, types::I32, [before, zero, relative])?;
    let key = reader.constant(types::I32, i64::from(ty))?;
    let result =
        reader.call(refs.find, &[key, root, cursor, input.reverse, input.ctx], types::I64)?;
    returned_child(world, reader, result, off)
}
pub(super) fn returned_child<'function>(
    world: &mut World<'function>,
    mut reader: Cursor<'function>,
    value: Value,
    off: Value,
) -> Result<Block, Diagnostic> {
    let absent = reader.constant(types::I64, -1)?;
    let has = reader.compare(IntCC::NotEqual, [value, absent])?;
    let (yes, next) = reader.branch(has)?;
    let mut found = world.at(yes, &[])?;
    let low = found.unary(O::Ireduce, types::I32, value)?;
    let offset = found.binary(O::Iadd, types::I32, [low, off])?;
    let mask = found.constant(types::I64, 0xffff_ffff_0000_0000u64.cast_signed())?;
    let high = found.binary(O::Band, types::I64, [value, mask])?;
    let low = found.unary(O::Uextend, types::I64, offset)?;
    let result = found.binary(O::Bor, types::I64, [high, low])?;
    found.returned(&[result])?;
    Ok(next)
}
