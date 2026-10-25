//! Vec outer effects; old backing retirement is part of Reserve, never a new event.
use super::{
    A, Diagnostic, FuncRef, FunctionBuilder, InstBuilder, IntCC, L, MemFlagsData, O, S, State,
    TrapCode, Value, checked, constant, invariant, status, store, types,
};
use zryna_native_mir::generic_execution_v1::VectorEffect;
use zryna_ownership_runtime_abi::{MAX_DYNAMIC_ALLOCATION_BYTES, MAX_VEC_ELEMENTS};
mod classify;
pub(super) fn element(
    b: &FunctionBuilder<'_>,
    effect: VectorEffect<'_, '_>,
    args: &[Value],
) -> Result<(), Diagnostic> {
    use cranelift_codegen::ir::{InstructionData, Opcode};
    let value = *args.first().ok_or_else(invariant)?;
    let inst = b.func.dfg.value_def(value).inst().ok_or_else(invariant)?;
    if !matches!(b.func.dfg.insts[inst], InstructionData::UnaryImm {
        opcode: Opcode::Iconst, imm } if i64::from(imm) == i64::from(effect.element()))
        || b.func.dfg.value_type(value) != types::I32
    {
        return Err(invariant());
    }
    Ok(())
}
mod acquire;
mod release;
/// The caller uses merged source status; the existing Push proof retains the actual C result.
pub(in crate::generic_execution_v1) struct Call {
    pub status: Value,
    pub canonical: Value,
}
pub(in crate::generic_execution_v1) use acquire::call;
pub(in crate::generic_execution_v1) use release::release;
