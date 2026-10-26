//! Closed selected-invocation recipe; output publication follows all cleanup/lease checks.
use super::{Cursor, finish, guard};
use crate::generic_execution_v1::invariant;
use cranelift_codegen::{
    ir::{AbiParam, FuncRef, Function, Opcode as O, Signature, condcodes::IntCC, types},
    isa::CallConv,
};
use zryna_abi::{ScalarType, ScalarValue, VerifiedInvocation};
use zryna_diagnostics::Diagnostic;
pub(in crate::generic_execution_v1) fn check(
    f: &Function,
    request: &VerifiedInvocation<'_>,
    callee: FuncRef,
    getter: Option<FuncRef>,
) -> Result<(), Diagnostic> {
    let mut signature = Signature::new(CallConv::SystemV);
    signature.params.push(AbiParam::new(types::I64));
    signature.returns.push(AbiParam::new(types::I32));
    if f.signature != signature
        || f.sized_stack_slots.len() != request.arguments().len() + 2
        || !f.dynamic_stack_slots.is_empty()
    {
        return Err(invariant());
    }
    let entry = f.layout.entry_block().ok_or_else(invariant)?;
    if f.layout.next_block(entry).is_some() {
        return Err(invariant());
    }
    let parameters = f.dfg.block_params(entry);
    if parameters.len() != 1 || f.dfg.value_type(parameters[0]) != types::I64 {
        return Err(invariant());
    }
    let host = parameters[0];
    let mut c = Cursor::new(f, entry);
    let zero = c.constant(types::I32, 0)?;
    c.store(host, 0, zero)?;
    let output = c.stack(8, 3)?;
    let zero = c.constant(types::I64, 0)?;
    c.store(output, 0, zero)?;
    let mut args = Vec::new();
    args.try_reserve_exact(request.arguments().len() + 2)
        .map_err(crate::generic_execution_v1::codegen)?;
    args.push(output);
    for value in request.arguments() {
        let address = c.stack(8, 3)?;
        let zero = c.constant(types::I64, 0)?;
        c.store(address, 0, zero)?;
        let value = match value {
            ScalarValue::Bool(value) => c.constant(types::I8, i64::from(*value))?,
            ScalarValue::I32(value) => c.constant(types::I32, i64::from(value.cast_unsigned()))?,
        };
        c.store(address, 0, value)?;
        args.push(address);
    }
    let ctx = c.stack(32, 3)?;
    let zero = c.constant(types::I64, 0)?;
    for offset in [0, 8, 16, 24] {
        c.store(ctx, offset, zero)?;
    }
    let arena = getter.map(|getter| super::super::lease::begin(&mut c, getter, ctx)).transpose()?;
    args.push(ctx);
    let status = c.call(callee, &args, types::I32)?;
    if let Some(base) = arena {
        super::super::lease::completed(&mut c, base, ctx)?;
    }
    finish::check(&mut c, ctx)?;
    if let Some(base) = arena {
        super::super::lease::end(&mut c, base)?;
    }
    let five = c.constant(types::I32, 5)?;
    let out_of_range = c.compare(IntCC::UnsignedGreaterThan, [status, five])?;
    let invariant = c.constant(types::I32, 255)?;
    let not_invariant = c.compare(IntCC::NotEqual, [status, invariant])?;
    let invalid = c.binary(O::Band, types::I8, [out_of_range, not_invariant])?;
    c.trap(invalid)?;
    let value = if request.export().result() == ScalarType::Bool {
        let value = c.load(types::I8, output, 0)?;
        let value = c.unary(O::Uextend, types::I32, value)?;
        guard(&mut c, value)?;
        value
    } else {
        c.load(types::I32, output, 0)?
    };
    let zero = c.constant(types::I32, 0)?;
    let success = c.compare(IntCC::Equal, [status, zero])?;
    let zero = c.constant(types::I32, 0)?;
    let value = c.ternary(O::Select, types::I32, [success, value, zero])?;
    c.store(host, 0, value)?;
    c.returned(&[status])
}
