//! Exact read-only owner/used gate; the standalone lifecycle matcher owns every instruction.
use super::super::calls::Refs;
use super::util::{c32, call, cmp, load, trap};
use cranelift_codegen::ir::{InstBuilder, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
pub(super) fn emit(
    b: &mut FunctionBuilder<'_>,
    r: Refs,
    ctx: Value,
) -> Result<(), zryna_diagnostics::Diagnostic> {
    let base = call(b, r.getter, &[ctx]);
    let used = load(b, types::I32, base, 0);
    let busy = load(b, types::I32, base, 4);
    let owner = load(b, types::I64, base, 8);
    let limit = c32(
        b,
        u32::try_from(zryna_ownership_runtime_abi::MAX_LIVE_ALLOCATIONS)
            .map_err(|_| crate::generic_execution_v1::invariant())?,
    );
    let excess = cmp(b, IntCC::UnsignedGreaterThan, used, limit);
    let one = c32(b, 1);
    let inactive = cmp(b, IntCC::NotEqual, busy, one);
    let foreign = cmp(b, IntCC::NotEqual, owner, ctx);
    let bad = b.ins().bor(excess, inactive);
    let bad = b.ins().bor(bad, foreign);
    trap(b, bad);
    b.ins().return_(&[base]);
    Ok(())
}
