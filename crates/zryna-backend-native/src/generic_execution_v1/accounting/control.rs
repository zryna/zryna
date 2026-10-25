//! NEW control8 outer-event accounting; canonical C signatures remain unchanged.
use super::{
    A, Diagnostic, FuncRef, FunctionBuilder, InstBuilder, IntCC, L, MemFlagsData, O, S, State,
    TrapCode, Value, checked, constant, invariant, store, types,
};
use zryna_native_mir::generic_execution_v1::ControlEffect;
use zryna_ownership_runtime_abi::MAX_DYNAMIC_ALLOCATION_BYTES;
mod acquire;
mod classify;
mod release;
pub(in crate::generic_execution_v1) use acquire::call;
pub(in crate::generic_execution_v1) use release::release;
fn arguments(
    b: &FunctionBuilder<'_>,
    effect: ControlEffect<'_, '_>,
    args: &[Value],
) -> Result<(), Diagnostic> {
    use cranelift_codegen::ir::{InstructionData, Opcode};
    let constant = |value: Value, expected: u64| {
        b.func.dfg.value_def(value).inst().is_some_and(|inst| matches!(b.func.dfg.insts[inst],
            InstructionData::UnaryImm { opcode: Opcode::Iconst, imm } if i64::from(imm) == expected.cast_signed()))
    };
    let valid = match effect.operation() {
        O::Allocate => {
            args.len() == 3
                && effect.size() >= 8
                && effect.alignment() >= 4
                && constant(args[0], effect.size())
                && constant(args[1], effect.alignment())
                && b.func.dfg.value_type(args[0]) == types::I64
                && b.func.dfg.value_type(args[1]) == types::I32
        }
        O::StrongClone
        | O::WeakDowngrade
        | O::WeakClone
        | O::WeakUpgrade
        | O::StrongReleaseFinish => args.len() == 1,
        O::StrongReleaseBegin | O::WeakRelease => args.len() == 2,
        _ => false,
    };
    if !valid {
        return Err(invariant());
    }
    Ok(())
}
fn canonical_status(b: &mut FunctionBuilder<'_>, value: Value, operation: O) {
    let mut valid = b.ins().icmp_imm_u(IntCC::Equal, value, 0);
    let codes: &[i64] = if operation == O::Allocate {
        &[1, 2]
    } else if operation == O::WeakUpgrade {
        &[3, 5]
    } else {
        &[3]
    };
    for code in codes {
        let bit = b.ins().icmp_imm_u(IntCC::Equal, value, *code);
        valid = b.ins().bor(valid, bit);
    }
    b.ins().trapz(valid, TrapCode::unwrap_user(7));
}
