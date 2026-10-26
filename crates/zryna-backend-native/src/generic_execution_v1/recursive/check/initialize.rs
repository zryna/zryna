//! Independent pure initializer CFG/byte recipe and genuine Clone-child authority.
use super::super::calls::{Kind, Refs, signature};
use super::{cursor::Cursor, world::World};
use crate::generic_execution_v1::{codegen, invariant};
use cranelift_codegen::ir::{Function, Value, condcodes::IntCC, types};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::CloneNodeKind;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
mod array;
mod bytes;
mod members;
#[derive(Clone, Copy)]
pub(super) struct Input {
    pub source: Value,
    pub destination: Value,
    pub context: Value,
}
pub(super) fn check(
    program: &VerifiedProgram<'_, '_>,
    function: &Function,
    refs: Refs,
) -> Result<(), Diagnostic> {
    if function.signature != signature(Kind::Init)
        || !function.sized_stack_slots.is_empty()
        || !function.dynamic_stack_slots.is_empty()
    {
        return Err(invariant());
    }
    let entry = function.layout.entry_block().ok_or_else(invariant)?;
    let mut world = World::new(function);
    let mut reader = world.at(entry, &[types::I32, types::I64, types::I64, types::I64])?;
    let args = function.dfg.block_params(entry);
    let input = Input { source: args[1], destination: args[2], context: args[3] };
    let plan = program.containers().ok_or_else(invariant)?;
    for index in 0..program.program().type_count() {
        let ty = u32::try_from(index).map_err(codegen)?;
        let Some(node) = plan.clone_node(ty) else {
            continue;
        };
        let key = reader.constant(types::I32, i64::from(ty))?;
        let selected = reader.compare(IntCC::Equal, [args[0], key])?;
        let (active, next) = reader.branch(selected)?;
        let active = world.at(active, &[])?;
        let size = u32::try_from(program.stored_type(ty).ok_or_else(invariant)?.size())
            .map_err(codegen)?;
        let active = bytes::check(&mut world, active, None, input.destination, size)?;
        let mut active = match node.kind() {
            CloneNodeKind::Copy => {
                bytes::check(&mut world, active, Some(input.source), input.destination, size)?
            }
            CloneNodeKind::Struct | CloneNodeKind::Enum => {
                members::check(program, &mut world, active, refs, ty, input)?
            }
            CloneNodeKind::FixedArray { length } => {
                array::check(program, &mut world, active, refs, (ty, input, length))?
            }
            CloneNodeKind::String
            | CloneNodeKind::VecEach
            | CloneNodeKind::SharedCount
            | CloneNodeKind::WeakCount => active,
        };
        let zero = active.constant(types::I32, 0)?;
        active.returned(&[zero])?;
        reader = world.at(next, &[])?;
    }
    reader.unreachable()?;
    world.finish()
}
pub(super) fn child(
    reader: &mut Cursor<'_>,
    refs: Refs,
    ty: u32,
    source: Value,
    destination: Value,
    context: Value,
) -> Result<(), Diagnostic> {
    let key = reader.constant(types::I32, i64::from(ty))?;
    let status = reader.call(refs.init, &[key, source, destination, context], types::I32)?;
    reader.trap(status)
}
