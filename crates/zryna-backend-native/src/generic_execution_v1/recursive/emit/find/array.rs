//! Checked physical Array cursor skips completed elements; both directions share one child call.
use super::super::util::{self as u, c32, call, cmp};
use super::{Input, Refs, none, returned_child};
use crate::generic_execution_v1::{codegen, invariant};
use cranelift_codegen::ir::{InstBuilder, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::generic_execution_v1::VerifiedProgram;
pub(super) fn emit(
    p: &VerifiedProgram<'_, '_>,
    b: &mut FunctionBuilder<'_>,
    r: Refs,
    id: u32,
    i: Input,
) -> Result<(), Diagnostic> {
    let view = p.stored_type(id).ok_or_else(invariant)?;
    let length = u32::try_from(view.array_length().ok_or_else(invariant)?).map_err(codegen)?;
    let stride = u32::try_from(view.array_stride().ok_or_else(invariant)?).map_err(codegen)?;
    if length == 0 || stride == 0 {
        none(b);
        return Ok(());
    }
    let reverse = b.create_block();
    let forward = b.create_block();
    let repeat = u::block(b, &[types::I32]);
    let inspect = b.create_block();
    let done = b.create_block();
    let zero = c32(b, 0);
    let backwards = cmp(b, IntCC::NotEqual, i.reverse, zero);
    b.ins().brif(backwards, reverse, &[], forward, &[]);
    b.switch_to_block(reverse);
    let empty = cmp(b, IntCC::Equal, i.cursor, zero);
    let start = b.create_block();
    b.ins().brif(empty, done, &[], start, &[]);
    b.switch_to_block(start);
    let one = c32(b, 1);
    let prev = b.ins().isub(i.cursor, one);
    let width = c32(b, stride);
    let first = b.ins().udiv(prev, width);
    let last = c32(b, length - 1);
    let past = cmp(b, IntCC::UnsignedGreaterThan, first, last);
    let first = b.ins().select(past, last, first);
    b.ins().jump(repeat, &[first.into()]);
    b.switch_to_block(forward);
    let width = c32(b, stride);
    let first = b.ins().udiv(i.cursor, width);
    b.ins().jump(repeat, &[first.into()]);
    b.switch_to_block(repeat);
    let index = b.block_params(repeat)[0];
    let count = c32(b, length);
    let past = cmp(b, IntCC::UnsignedGreaterThanOrEqual, index, count);
    b.ins().brif(past, done, &[], inspect, &[]);
    b.switch_to_block(inspect);
    let width = c32(b, stride);
    let offset = b.ins().imul(index, width);
    let root = u::offset(b, i.root, offset);
    let zero = c32(b, 0);
    let early = cmp(b, IntCC::UnsignedLessThan, i.cursor, offset);
    let relative = b.ins().isub(i.cursor, offset);
    let cursor = b.ins().select(early, zero, relative);
    let child = c32(b, view.referenced_type().ok_or_else(invariant)?.index());
    let result = call(b, r.find, &[child, root, cursor, i.reverse, i.ctx]);
    let next = b.create_block();
    returned_child(b, result, offset, next);
    b.switch_to_block(next);
    let reverse_next = b.create_block();
    let forward_next = b.create_block();
    b.ins().brif(backwards, reverse_next, &[], forward_next, &[]);
    b.switch_to_block(reverse_next);
    let empty = cmp(b, IntCC::Equal, index, zero);
    let previous = b.create_block();
    b.ins().brif(empty, done, &[], previous, &[]);
    b.switch_to_block(previous);
    let one = c32(b, 1);
    let value = b.ins().isub(index, one);
    b.ins().jump(repeat, &[value.into()]);
    b.switch_to_block(forward_next);
    let one = c32(b, 1);
    let value = b.ins().iadd(index, one);
    b.ins().jump(repeat, &[value.into()]);
    b.switch_to_block(done);
    none(b);
    Ok(())
}
