//! Fixed physical-byte loop, with no allocation or canonical effect.
use super::super::util::{self as u, c32, cmp};
use cranelift_codegen::ir::{InstBuilder, MemFlagsData, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
pub(super) fn emit(
    builder: &mut FunctionBuilder<'_>,
    source: Option<Value>,
    destination: Value,
    size: u32,
) {
    if size == 0 {
        return;
    }
    let repeat = u::block(builder, &[types::I32]);
    let body = builder.create_block();
    let done = builder.create_block();
    let zero = c32(builder, 0);
    builder.ins().jump(repeat, &[zero.into()]);
    builder.switch_to_block(repeat);
    let index = builder.block_params(repeat)[0];
    let count = c32(builder, size);
    let more = cmp(builder, IntCC::UnsignedLessThan, index, count);
    builder.ins().brif(more, body, &[], done, &[]);
    builder.switch_to_block(body);
    let offset = builder.ins().uextend(types::I64, index);
    let value = if let Some(source) = source {
        let address = builder.ins().iadd(source, offset);
        builder.ins().load(types::I8, MemFlagsData::new(), address, 0)
    } else {
        builder.ins().iconst(types::I8, 0)
    };
    let target = builder.ins().iadd(destination, offset);
    builder.ins().store(MemFlagsData::new(), value, target, 0);
    let one = c32(builder, 1);
    let next = builder.ins().iadd(index, one);
    builder.ins().jump(repeat, &[next.into()]);
    builder.switch_to_block(done);
}
