//! Invocation-local initialized element counts; failed children unwind themselves first.
use super::Helper;
use cranelift_codegen::ir::{InstBuilder, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
pub(super) fn drop(
    h: &Helper<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    child: u32,
    data: Value,
    length: Value,
    stride: u64,
) -> Result<(), Diagnostic> {
    let header = b.create_block();
    let body = b.create_block();
    let done = b.create_block();
    b.append_block_param(header, types::I64);
    b.ins().jump(header, &[length.into()]);
    b.switch_to_block(header);
    let remaining = b.block_params(header)[0];
    let any = b.ins().icmp_imm_u(IntCC::NotEqual, remaining, 0);
    b.ins().brif(any, body, &[], done, &[]);
    b.switch_to_block(body);
    let index = b.ins().iadd_imm_s(remaining, -1);
    let offset = b.ins().imul_imm_u(index, i64::try_from(stride).map_err(super::codegen)?);
    let address = b.ins().iadd(data, offset);
    h.refs.drop_value(h.p, b, child, address, h.invocation)?;
    b.ins().jump(header, &[index.into()]);
    b.switch_to_block(done);
    Ok(())
}
#[derive(Clone, Copy)]
pub(super) struct Pair {
    pub source: Value,
    pub destination: Value,
    pub length: Value,
    pub stride: u64,
    pub child: u32,
    pub buffer: Option<Value>,
}
pub(super) fn clone(
    h: &Helper<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    pair: Pair,
) -> Result<(), Diagnostic> {
    let header = b.create_block();
    let body = b.create_block();
    let done = b.create_block();
    let failed = b.create_block();
    b.append_block_param(header, types::I64);
    b.append_block_param(failed, types::I32);
    b.append_block_param(failed, types::I64);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().jump(header, &[zero.into()]);
    b.switch_to_block(header);
    let index = b.block_params(header)[0];
    let more = b.ins().icmp(IntCC::UnsignedLessThan, index, pair.length);
    b.ins().brif(more, body, &[], done, &[]);
    b.switch_to_block(body);
    let offset = b.ins().imul_imm_u(index, i64::try_from(pair.stride).map_err(super::codegen)?);
    let source = b.ins().iadd(pair.source, offset);
    let destination = b.ins().iadd(pair.destination, offset);
    let status = h.refs.clone_value(h.p, b, pair.child, source, destination, h.invocation)?;
    let next = b.create_block();
    b.ins().brif(status, failed, &[status.into(), index.into()], next, &[]);
    b.switch_to_block(next);
    let initialized = b.ins().iadd_imm_u(index, 1);
    b.ins().jump(header, &[initialized.into()]);
    b.switch_to_block(failed);
    let status = b.block_params(failed)[0];
    let initialized = b.block_params(failed)[1];
    drop(h, b, pair.child, pair.destination, initialized, pair.stride)?;
    // Vec helpers additionally release their own buffer before returning status; arrays do not.
    if let Some(buffer) = pair.buffer {
        super::cloning::release_vec(h, b, buffer)?;
    }
    b.ins().return_(&[status]);
    b.switch_to_block(done);
    Ok(())
}
