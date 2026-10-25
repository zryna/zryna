//! Capture deallocation/last-owner facts before C can free control; never reread afterward.
use super::{
    Diagnostic, FunctionBuilder, InstBuilder, IntCC, MemFlagsData, O, TrapCode, Value, constant,
    invariant, types,
};
pub(super) struct Facts {
    pub amount: Value,
    expected: Option<Value>,
}
pub(super) fn capture(
    b: &mut FunctionBuilder<'_>,
    operation: O,
    args: &[Value],
) -> Result<Facts, Diagnostic> {
    let zero = constant(b, 0);
    let (amount, expected) = match operation {
        O::StrongReleaseBegin => {
            let count = b.ins().load(types::I32, MemFlagsData::new(), args[0], 0);
            let last = b.ins().icmp_imm_u(IntCC::Equal, count, 1);
            (zero, Some(last))
        }
        O::StrongReleaseFinish => {
            let count = b.ins().load(types::I32, MemFlagsData::new(), args[0], 4);
            let last = b.ins().icmp_imm_u(IntCC::Equal, count, 1);
            (b.ins().uextend(types::I64, last), None)
        }
        O::WeakRelease => {
            let strong = b.ins().load(types::I32, MemFlagsData::new(), args[0], 0);
            let weak = b.ins().load(types::I32, MemFlagsData::new(), args[0], 4);
            let expired = b.ins().icmp_imm_u(IntCC::Equal, strong, 0);
            let last = b.ins().icmp_imm_u(IntCC::Equal, weak, 1);
            let free = b.ins().band(expired, last);
            (b.ins().uextend(types::I64, free), Some(free))
        }
        _ => return Err(invariant()),
    };
    if expected.is_some() {
        let zero = b.ins().iconst(types::I32, 0);
        b.ins().store(MemFlagsData::new(), zero, args[1], 0);
    }
    Ok(Facts { amount, expected })
}
pub(super) fn validate(b: &mut FunctionBuilder<'_>, facts: &Facts, args: &[Value]) {
    if let Some(expected) = facts.expected {
        let actual = b.ins().load(types::I32, MemFlagsData::new(), args[1], 0);
        let expected = b.ins().uextend(types::I32, expected);
        let bad = b.ins().icmp(IntCC::NotEqual, actual, expected);
        b.ins().trapnz(bad, TrapCode::unwrap_user(7));
    }
}
