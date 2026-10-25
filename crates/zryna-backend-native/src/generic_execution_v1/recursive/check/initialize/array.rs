//! Certified array child, exact physical stride and initialized inline cursor progress.
use super::{Cursor, Input, Refs, World, child};
use crate::generic_execution_v1::{codegen, invariant};
use cranelift_codegen::ir::{Opcode as O, condcodes::IntCC, types};
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
pub(super) fn check<'f>(
    program: &VerifiedProgram<'_, '_>,
    world: &mut World<'f>,
    mut reader: Cursor<'f>,
    refs: Refs,
    target: (u32, Input, u64),
) -> Result<Cursor<'f>, Diagnostic> {
    let (ty, input, length) = target;
    if length == 0 {
        return Ok(reader);
    }
    let view = program.stored_type(ty).ok_or_else(invariant)?;
    let node = program.containers().and_then(|p| p.clone_node(ty)).ok_or_else(invariant)?;
    let child_ty = node.element().ok_or_else(invariant)?.ty();
    if view.array_length() != Some(length)
        || view.referenced_type().map(zryna_layout::generic_v1::TypeId::index) != Some(child_ty)
    {
        return Err(invariant());
    }
    let zero = reader.constant(types::I32, 0)?;
    let repeat = reader.jump(&[zero])?;
    let mut header = world.at(repeat, &[types::I32])?;
    let index = world.f.dfg.block_params(repeat)[0];
    let count = header.constant(types::I32, i64::from(u32::try_from(length).map_err(codegen)?))?;
    let more = header.compare(IntCC::UnsignedLessThan, [index, count])?;
    let (body, done) = header.branch(more)?;
    let mut body = world.at(body, &[])?;
    let wide = body.unary(O::Uextend, types::I64, index)?;
    let stride =
        body.constant(types::I64, view.array_stride().ok_or_else(invariant)?.cast_signed())?;
    let offset = body.binary(O::Imul, types::I64, [wide, stride])?;
    let source = body.binary(O::Iadd, types::I64, [input.source, offset])?;
    let destination = body.binary(O::Iadd, types::I64, [input.destination, offset])?;
    child(&mut body, refs, child_ty, source, destination, input.context)?;
    let one = body.constant(types::I32, 1)?;
    let next = body.binary(O::Iadd, types::I32, [index, one])?;
    if body.jump(&[next])? != repeat {
        return Err(invariant());
    }
    world.at(done, &[])
}
