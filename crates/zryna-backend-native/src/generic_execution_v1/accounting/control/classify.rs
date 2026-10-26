//! Prospective successful-owner debt from the actual canonical call operands/counts.
use super::{
    FunctionBuilder, InstBuilder, IntCC, MAX_DYNAMIC_ALLOCATION_BYTES, MemFlagsData, O, State,
    TrapCode, Value, constant, types,
};
pub(super) struct Classified {
    pub possible: Value,
    pub attempt: Value,
    pub debt: Value,
}
pub(super) fn classify(b: &mut FunctionBuilder<'_>, operation: O, args: &[Value]) -> Classified {
    let zero = constant(b, 0);
    let possible = if operation == O::Allocate {
        b.ins().icmp_imm_u(
            IntCC::UnsignedLessThanOrEqual,
            args[0],
            MAX_DYNAMIC_ALLOCATION_BYTES.cast_signed(),
        )
    } else {
        let offset = if matches!(operation, O::WeakDowngrade | O::WeakClone) { 4 } else { 0 };
        let count = b.ins().load(types::I32, MemFlagsData::new(), args[0], offset);
        let maximum = b.ins().iconst(types::I32, -1);
        let room = b.ins().icmp(IntCC::NotEqual, count, maximum);
        if operation == O::WeakUpgrade {
            let alive = b.ins().icmp_imm_u(IntCC::NotEqual, count, 0);
            b.ins().band(room, alive)
        } else {
            room
        }
    };
    let held = b.ins().uextend(types::I64, possible);
    if operation == O::Allocate {
        Classified { possible, attempt: held, debt: b.ins().iadd(held, held) }
    } else {
        Classified { possible, attempt: zero, debt: held }
    }
}
pub(super) fn funded(b: &mut FunctionBuilder<'_>, old: State) {
    let no_held = b.ins().icmp_imm_u(IntCC::Equal, old.h, 0);
    let no_live = b.ins().icmp_imm_u(IntCC::Equal, old.l, 0);
    let bad = b.ins().bor(no_held, no_live);
    b.ins().trapnz(bad, TrapCode::unwrap_user(7));
}
