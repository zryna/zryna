//! Exactly the canonical C capacity classifier, with a fixed twenty-step expansion.
use super::{
    Diagnostic, FunctionBuilder, InstBuilder, IntCC, MAX_DYNAMIC_ALLOCATION_BYTES,
    MAX_VEC_ELEMENTS, MemFlagsData, O, Value, VectorEffect, constant, invariant, types,
};
pub(super) struct Classification {
    pub possible: Value,
    pub attempt: Value,
    pub debt: Value,
    pub retired: Value,
}
pub(super) fn classify(
    b: &mut FunctionBuilder<'_>,
    effect: VectorEffect<'_, '_>,
    args: &[Value],
) -> Result<Classification, Diagnostic> {
    let zero = constant(b, 0);
    let limit = MAX_VEC_ELEMENTS.min(MAX_DYNAMIC_ALLOCATION_BYTES / effect.stride());
    let (capacity, growth, old_positive) = match effect.operation() {
        O::VecAllocate if args.len() == 3 => {
            let positive = b.ins().icmp_imm_u(IntCC::NotEqual, args[1], 0);
            (args[1], positive, zero)
        }
        O::VecReserve if args.len() == 4 => {
            let old = b.ins().load(types::I64, MemFlagsData::new(), args[1], 16);
            let growth = b.ins().icmp(IntCC::UnsignedGreaterThan, args[2], old);
            let positive = b.ins().icmp_imm_u(IntCC::NotEqual, old, 0);
            let retired = b.ins().band(growth, positive);
            let one = constant(b, 1);
            let empty = b.ins().icmp_imm_u(IntCC::Equal, old, 0);
            let mut capacity = b.ins().select(empty, one, old);
            // C can double at most20 times from1 to MAX_VEC_ELEMENTS=2^20.
            // An issued old capacity is in[0,MAX_VEC_ELEMENTS]; each doubled value
            // is at most2*MAX_VEC_ELEMENTS. No shift, multiplication or add wraps.
            for _ in 0..20 {
                let short = b.ins().icmp(IntCC::UnsignedLessThan, capacity, args[2]);
                let small = b.ins().icmp_imm_u(
                    IntCC::UnsignedLessThanOrEqual,
                    capacity,
                    (MAX_VEC_ELEMENTS / 2).cast_signed(),
                );
                let grow = b.ins().band(short, small);
                let twice = b.ins().iadd(capacity, capacity);
                capacity = b.ins().select(grow, twice, capacity);
            }
            let short = b.ins().icmp(IntCC::UnsignedLessThan, capacity, args[2]);
            capacity = b.ins().select(short, args[2], capacity);
            // The C no-growth path returns before the capacity classifier.
            capacity = b.ins().select(growth, capacity, old);
            (capacity, growth, retired)
        }
        _ => return Err(invariant()),
    };
    let possible =
        b.ins().icmp_imm_u(IntCC::UnsignedLessThanOrEqual, capacity, limit.cast_signed());
    let attempt = b.ins().band(possible, growth);
    let debt = if effect.operation() == O::VecAllocate {
        b.ins().uextend(types::I64, possible)
    } else {
        zero
    };
    let retired = if effect.operation() == O::VecAllocate {
        zero
    } else {
        b.ins().uextend(types::I64, old_positive)
    };
    Ok(Classification { possible, attempt: b.ins().uextend(types::I64, attempt), debt, retired })
}
