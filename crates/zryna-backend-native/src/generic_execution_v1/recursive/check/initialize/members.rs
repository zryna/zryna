//! Independent source-certified field/variant ordinal and physical offset matching.
use super::{Cursor, Input, Refs, World, child};
use crate::generic_execution_v1::{codegen, invariant};
use cranelift_codegen::ir::{Opcode as O, Value, condcodes::IntCC, types};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::CloneNodeKind;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
fn add(reader: &mut Cursor<'_>, root: Value, offset: u64) -> Result<Value, Diagnostic> {
    let offset = reader.constant(types::I64, offset.cast_signed())?;
    reader.binary(O::Iadd, types::I64, [root, offset])
}
pub(super) fn check<'f>(
    program: &VerifiedProgram<'_, '_>,
    world: &mut World<'f>,
    mut reader: Cursor<'f>,
    refs: Refs,
    ty: u32,
    input: Input,
) -> Result<Cursor<'f>, Diagnostic> {
    let node = program.containers().and_then(|p| p.clone_node(ty)).ok_or_else(invariant)?;
    let view = program.stored_type(ty).ok_or_else(invariant)?;
    if node.kind() == CloneNodeKind::Struct {
        if node.fields().len() != view.fields().len() {
            return Err(invariant());
        }
        for ((ordinal, entry), (physical, field, offset)) in node.fields().zip(view.fields()) {
            if ordinal != physical || entry.ty() != field.index() {
                return Err(invariant());
            }
            let source = add(&mut reader, input.source, offset)?;
            let destination = add(&mut reader, input.destination, offset)?;
            child(&mut reader, refs, entry.ty(), source, destination, input.context)?;
        }
        return Ok(reader);
    }
    if node.variants().len() != view.variants().len() {
        return Err(invariant());
    }
    let tag = reader.load(types::I32, input.source, 0)?;
    reader.store(input.destination, 0, tag)?;
    let mut done = None;
    for ((ordinal, entry), (physical, payload)) in node.variants().zip(view.variants()) {
        if ordinal != physical
            || entry.map(zryna_ir::generic_v1::execution_v1::VerifiedCloneNode::ty)
                != payload.map(zryna_layout::generic_v1::TypeId::index)
        {
            return Err(invariant());
        }
        let key = reader.constant(types::I32, i64::from(ordinal))?;
        let selected = reader.compare(IntCC::Equal, [tag, key])?;
        let (active, next) = reader.branch(selected)?;
        let mut active = world.at(active, &[])?;
        if let Some(entry) = entry {
            let offset =
                u64::from(u32::try_from(view.payload().ok_or_else(invariant)?.0).map_err(codegen)?);
            let source = add(&mut active, input.source, offset)?;
            let destination = add(&mut active, input.destination, offset)?;
            child(&mut active, refs, entry.ty(), source, destination, input.context)?;
        }
        let target = active.jump(&[])?;
        if done.is_some_and(|old| old != target) {
            return Err(invariant());
        }
        done = Some(target);
        reader = world.at(next, &[])?;
    }
    reader.unreachable()?;
    world.at(done.ok_or_else(invariant)?, &[])
}
