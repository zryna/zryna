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
    runtime: &Runtime<'_>,
    drops: &BTreeMap<u32, FuncRef>,
    failed: Block,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let observer = *runtime.get(OBSERVER).ok_or_else(super::invariant_error)?;
    let next = builder.create_block();
    builder.ins().jump(next, &[]);
    builder.switch_to_block(failed);
    let code = builder.block_params(failed)[0];
    super::drop::execute_cleanup_plan(program, function, cleanup, slots, runtime, drops, builder)?;
    builder.ins().call(observer, &[code]);
    let zero = builder.ins().iconst(super::lower::native_type(program, function.result_type())?, 0);
    builder.ins().return_(&[zero]);
    builder.switch_to_block(next);
    Ok(())
}

pub(super) struct Runtime<'a> {
    pub(super) symbols: &'a BTreeMap<&'a str, FuncRef>,
    pub(super) failed: Option<Block>,
}

impl<'a> std::ops::Deref for Runtime<'a> {
    type Target = BTreeMap<&'a str, FuncRef>;
    fn deref(&self) -> &Self::Target {
        self.symbols
    }
}

pub(super) fn runtime_status(
    runtime: &Runtime<'_>,
    status: Value,
    builder: &mut FunctionBuilder<'_>,
) {
    use cranelift_codegen::ir::{TrapCode, condcodes::IntCC};
    if let Some(failed) = runtime.failed {
        let invalid = builder.ins().icmp_imm_u(IntCC::UnsignedGreaterThan, status, 4);
        builder.ins().trapnz(invalid, TrapCode::unwrap_user(2));
        let code = builder.ins().iadd_imm_u(status, 1);
        let next = builder.create_block();
        builder.ins().brif(status, failed, &[BlockArg::Value(code)], next, &[]);
        builder.switch_to_block(next);
    } else {
        builder.ins().trapnz(status, TrapCode::unwrap_user(2));
    }
}

pub(super) fn language_status(
    runtime: &Runtime<'_>,
    status: Value,
    builder: &mut FunctionBuilder<'_>,
) {
    if let Some(failed) = runtime.failed {
        let next = builder.create_block();
        builder.ins().brif(status, failed, &[BlockArg::Value(status)], next, &[]);
        builder.switch_to_block(next);
    } else {
        builder.ins().trapnz(status, cranelift_codegen::ir::TrapCode::unwrap_user(2));
    }
}

pub(super) fn check_propagated(
    runtime: &Runtime<'_>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let observer = *runtime.get(OBSERVER).ok_or_else(super::invariant_error)?;
    let zero = builder.ins().iconst(types::I32, 0);
    let call = builder.ins().call(observer, &[zero]);
    language_status(runtime, builder.inst_results(call)[0], builder);
    Ok(())
}

pub(super) fn return_trap(
    program: &VerifiedMirModule,
    function: VerifiedFunction<'_>,
    code: Value,
    runtime: &Runtime<'_>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let observer = *runtime.get(OBSERVER).ok_or_else(super::invariant_error)?;
    builder.ins().call(observer, &[code]);
    let zero = builder.ins().iconst(super::lower::native_type(program, function.result_type())?, 0);
    builder.ins().return_(&[zero]);
    Ok(())
}

pub(super) fn record(
    word: u32,
    runtime: &Runtime<'_>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let observer = *runtime.get(OBSERVER).ok_or_else(super::invariant_error)?;
    let command = builder.ins().iconst(types::I32, i64::from(0x8000_0000_u32 | word));
    builder.ins().call(observer, &[command]);
    Ok(())
}

pub(super) fn probe(
    code: u32,
    runtime: &Runtime<'_>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let observer = *runtime.get(OBSERVER).ok_or_else(super::invariant_error)?;
    let command = builder.ins().iconst(types::I32, i64::from(0x1000_0000_u32 + code));
    let call = builder.ins().call(observer, &[command]);
    language_status(runtime, builder.inst_results(call)[0], builder);
    Ok(())
}

pub(super) fn value_kind(
    category: zryna_native_mir::data_ownership_v1::raw::TypeCategory,
) -> Option<u32> {
    use zryna_native_mir::data_ownership_v1::raw::TypeCategory as T;
    match category {
        T::Bool | T::I32 => None,
        T::String => Some(1),
        T::Struct | T::FixedArray | T::Vec => Some(2),
        T::Enum => Some(3),
        T::Shared => Some(4),
        T::Weak => Some(5),
    }
}

pub(super) fn record_if(
    condition: Value,
    word: u32,
    runtime: &Runtime<'_>,
    builder: &mut FunctionBuilder<'_>,
) -> Result<(), Diagnostic> {
    let record_block = builder.create_block();
    let next = builder.create_block();
    builder.ins().brif(condition, record_block, &[], next, &[]);
    builder.switch_to_block(record_block);
    record(word, runtime, builder)?;
    builder.ins().jump(next, &[]);
    builder.switch_to_block(next);
    Ok(())
}
