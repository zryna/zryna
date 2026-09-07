use cranelift_codegen::ir::{Block, BlockArg, FuncRef, InstBuilder, StackSlot, Value, types};
use cranelift_frontend::FunctionBuilder;
use std::collections::BTreeMap;
use zryna_diagnostics::Diagnostic;
use zryna_native_mir::data_ownership_v1::{VerifiedFunction, VerifiedMirModule};

pub(super) const OBSERVER: &str = "zryna_m3_observe";

pub(super) fn bounds(ok: Value, failed: Block, builder: &mut FunctionBuilder<'_>) {
    let next = builder.create_block();
    let code = builder.ins().iconst(types::I32, 1);
    builder.ins().brif(ok, next, &[], failed, &[BlockArg::Value(code)]);
    builder.switch_to_block(next);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn finish_operation(
    program: &VerifiedMirModule,
    function: VerifiedFunction<'_>,
    cleanup: Option<u32>,
    slots: &[Option<StackSlot>],
    runtime: &BTreeMap<&str, FuncRef>,
    drops: &BTreeMap<u32, FuncRef>,
    failed: Block,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let observer = *runtime.get(OBSERVER).ok_or_else(super::invariant_error)?;
    let zero = builder.ins().iconst(types::I32, 0);
    let call = builder.ins().call(observer, &[zero]);
    let status = builder.inst_results(call)[0];
    let next = builder.create_block();
    builder.ins().brif(status, failed, &[BlockArg::Value(status)], next, &[]);
    builder.switch_to_block(failed);
    let code = builder.block_params(failed)[0];
    super::drop::execute_cleanup_plan(program, function, cleanup, slots, runtime, drops, builder)?;
    builder.ins().call(observer, &[code]);
    let zero = builder.ins().iconst(super::lower::native_type(program, function.result_type())?, 0);
    builder.ins().return_(&[zero]);
    builder.switch_to_block(next);
    Ok(())
}
