//! Independent array cursor bounds, one shared recursive site and exact index progress.
use super::{Cursor, Input, Refs, World, none, returned_child};
use crate::generic_execution_v1::{codegen, invariant};
use cranelift_codegen::ir::{Opcode as O, condcodes::IntCC, types};
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
pub(super) fn check<'f>(
    program: &VerifiedProgram<'_, '_>,
    world: &mut World<'f>,
    mut reader: Cursor<'f>,
    refs: Refs,
    id: u32,
    input: Input,
) -> Result<(), Diagnostic> {
    let view = program.stored_type(id).ok_or_else(invariant)?;
    let length = u32::try_from(view.array_length().ok_or_else(invariant)?).map_err(codegen)?;
    let stride = u32::try_from(view.array_stride().ok_or_else(invariant)?).map_err(codegen)?;
    if length == 0 || stride == 0 {
        return none(&mut reader);
    }
    let zero = reader.constant(types::I32, 0)?;
    let backwards = reader.compare(IntCC::NotEqual, [input.reverse, zero])?;
    let (reverse, forward) = reader.branch(backwards)?;
    let mut reverse = world.at(reverse, &[])?;
    let empty = reverse.compare(IntCC::Equal, [input.cursor, zero])?;
    let (done, start) = reverse.branch(empty)?;
    let mut start = world.at(start, &[])?;
    let one = start.constant(types::I32, 1)?;
    let prev = start.binary(O::Isub, types::I32, [input.cursor, one])?;
    let width = start.constant(types::I32, i64::from(stride))?;
    let first = start.binary(O::Udiv, types::I32, [prev, width])?;
    let last = start.constant(types::I32, i64::from(length - 1))?;
    let past = start.compare(IntCC::UnsignedGreaterThan, [first, last])?;
    let first = start.ternary(O::Select, types::I32, [past, last, first])?;
    let repeat = start.jump(&[first])?;
    let mut forward = world.at(forward, &[])?;
    let width = forward.constant(types::I32, i64::from(stride))?;
    let first = forward.binary(O::Udiv, types::I32, [input.cursor, width])?;
    if forward.jump(&[first])? != repeat {
        return Err(invariant());
    }
    let mut body = world.at(repeat, &[types::I32])?;
    let index = world.f.dfg.block_params(repeat)[0];
    let count = body.constant(types::I32, i64::from(length))?;
    let past = body.compare(IntCC::UnsignedGreaterThanOrEqual, [index, count])?;
    let (end, inspect) = body.branch(past)?;
    if end != done {
        return Err(invariant());
    }
    let mut inspect = world.at(inspect, &[])?;
    let width = inspect.constant(types::I32, i64::from(stride))?;
    let offset = inspect.binary(O::Imul, types::I32, [index, width])?;
    let wide = inspect.unary(O::Uextend, types::I64, offset)?;
    let root = inspect.binary(O::Iadd, types::I64, [input.root, wide])?;
    let zero = inspect.constant(types::I32, 0)?;
    let early = inspect.compare(IntCC::UnsignedLessThan, [input.cursor, offset])?;
    let relative = inspect.binary(O::Isub, types::I32, [input.cursor, offset])?;
    let cursor = inspect.ternary(O::Select, types::I32, [early, zero, relative])?;
    let child = inspect
        .constant(types::I32, i64::from(view.referenced_type().ok_or_else(invariant)?.index()))?;
    let result =
        inspect.call(refs.find, &[child, root, cursor, input.reverse, input.ctx], types::I64)?;
    let next = returned_child(world, inspect, result, offset)?;
    let mut next = world.at(next, &[])?;
    let (back, front) = next.branch(backwards)?;
    let mut back = world.at(back, &[])?;
    let empty = back.compare(IntCC::Equal, [index, zero])?;
    let (end, previous) = back.branch(empty)?;
    if end != done {
        return Err(invariant());
    }
    let mut previous = world.at(previous, &[])?;
    let one = previous.constant(types::I32, 1)?;
    let prev = previous.binary(O::Isub, types::I32, [index, one])?;
    if previous.jump(&[prev])? != repeat {
        return Err(invariant());
    }
    let mut front = world.at(front, &[])?;
    let one = front.constant(types::I32, 1)?;
    let next = front.binary(O::Iadd, types::I32, [index, one])?;
    if front.jump(&[next])? != repeat {
        return Err(invariant());
    }
    none(&mut world.at(done, &[])?)
}
