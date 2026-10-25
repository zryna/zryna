//! Read-only leaf finder over the independently checked acyclic inline layout graph.
use super::super::calls::Refs;
use super::util::{self as u, c32, c64, call, cmp, none};
use crate::generic_execution_v1::{codegen, invariant};
use cranelift_codegen::ir::{Block, InstBuilder, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
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
pub(super) fn emit(
    p: &VerifiedProgram<'_, '_>,
    b: &mut FunctionBuilder<'_>,
    r: Refs,
    args: &[Value],
) -> Result<(), Diagnostic> {
    let input = Input { root: args[1], cursor: args[2], reverse: args[3], ctx: args[4] };
    for id in 0..p.program().type_count() {
        let id = u32::try_from(id).map_err(codegen)?;
        let yes = b.create_block();
        let next = b.create_block();
        let key = c32(b, id);
        let selected = cmp(b, IntCC::Equal, args[0], key);
        b.ins().brif(selected, yes, &[], next, &[]);
        b.switch_to_block(yes);
        let view = p.stored_type(id).ok_or_else(invariant)?;
        if view.size() == 0 {
            none(b);
        } else {
            match view.category() {
                TypeCategory::Bool | TypeCategory::I32 => none(b),
                TypeCategory::String
                | TypeCategory::Vec
                | TypeCategory::Shared
                | TypeCategory::Weak => {
                    terminal(b, id, input);
                }
                TypeCategory::FixedArray => array::emit(p, b, r, id, input)?,
                TypeCategory::Struct | TypeCategory::Enum => members::emit(p, b, r, id, input)?,
            }
        }
        b.switch_to_block(next);
    }
    b.ins().trap(cranelift_codegen::ir::TrapCode::unwrap_user(7));
    Ok(())
}
fn terminal(b: &mut FunctionBuilder<'_>, id: u32, i: Input) {
    let yes = b.create_block();
    let no = b.create_block();
    let zero = c32(b, 0);
    let reversing = cmp(b, IntCC::NotEqual, i.reverse, zero);
    let backward = cmp(b, IntCC::NotEqual, i.cursor, zero);
    let forward = cmp(b, IntCC::Equal, i.cursor, zero);
    let found = b.ins().select(reversing, backward, forward);
    b.ins().brif(found, yes, &[], no, &[]);
    b.switch_to_block(yes);
    let result = c64(b, u64::from(id) << 32);
    b.ins().return_(&[result]);
    b.switch_to_block(no);
    none(b);
}
pub(super) fn child(
    b: &mut FunctionBuilder<'_>,
    r: Refs,
    child: u32,
    offset: u32,
    i: Input,
    next: Block,
) {
    let off = c32(b, offset);
    let root = u::offset(b, i.root, off);
    let zero = c32(b, 0);
    let before = cmp(b, IntCC::UnsignedLessThan, i.cursor, off);
    let relative = b.ins().isub(i.cursor, off);
    let cursor = b.ins().select(before, zero, relative);
    let key = c32(b, child);
    let result = call(b, r.find, &[key, root, cursor, i.reverse, i.ctx]);
    returned_child(b, result, off, next);
}
pub(super) fn returned_child(b: &mut FunctionBuilder<'_>, result: Value, off: Value, next: Block) {
    let found = b.create_block();
    let absent = c64(b, u64::MAX);
    let has = cmp(b, IntCC::NotEqual, result, absent);
    b.ins().brif(has, found, &[], next, &[]);
    b.switch_to_block(found);
    let low = b.ins().ireduce(types::I32, result);
    let offset = b.ins().iadd(low, off);
    let value = u::packed(b, result, offset);
    b.ins().return_(&[value]);
}
