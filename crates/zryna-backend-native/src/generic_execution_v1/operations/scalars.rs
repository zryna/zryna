//! Wrapping scalar operations and exact scalar-width stores.
use super::{
    super::{body::Frame, invariant},
    scalar,
};
use cranelift_codegen::ir::{InstBuilder, MemFlagsData, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{VerifiedOperation, raw};
pub(super) fn emit(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    op: VerifiedOperation<'_, '_>,
    result: Option<Value>,
) -> Result<(), Diagnostic> {
    match op.kind() {
        raw::Operation::BoolLiteral(value) => {
            let value = b.ins().iconst(types::I8, i64::from(*value));
            b.ins().store(MemFlagsData::new(), value, result.ok_or_else(invariant)?, 0);
        }
        raw::Operation::I32Literal(value) => {
            let value = b.ins().iconst(types::I32, i64::from(*value));
            b.ins().store(MemFlagsData::new(), value, result.ok_or_else(invariant)?, 0);
        }
        raw::Operation::I32Neg { value } => {
            let source = scalar(c, b, *value)?;
            let value = b.ins().ineg(source);
            store_scalar(c, b, op, value)?;
        }
        raw::Operation::ScalarBinary { kind, left, right } => {
            let left = scalar(c, b, *left)?;
            let right = scalar(c, b, *right)?;
            let value = match kind {
                raw::ScalarBinaryKind::Add => b.ins().iadd(left, right),
                raw::ScalarBinaryKind::Sub => b.ins().isub(left, right),
                raw::ScalarBinaryKind::Mul => b.ins().imul(left, right),
                _ => {
                    let cc = match kind {
                        raw::ScalarBinaryKind::Eq => IntCC::Equal,
                        raw::ScalarBinaryKind::Ne => IntCC::NotEqual,
                        raw::ScalarBinaryKind::LtS => IntCC::SignedLessThan,
                        raw::ScalarBinaryKind::LeS => IntCC::SignedLessThanOrEqual,
                        raw::ScalarBinaryKind::GtS => IntCC::SignedGreaterThan,
                        raw::ScalarBinaryKind::GeS => IntCC::SignedGreaterThanOrEqual,
                        _ => return Err(invariant()),
                    };
                    let test = b.ins().icmp(cc, left, right);
                    b.ins().uextend(types::I32, test)
                }
            };
            store_scalar(c, b, op, value)?;
        }
        _ => return Err(invariant()),
    }
    Ok(())
}
fn store_scalar(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    op: VerifiedOperation<'_, '_>,
    value: Value,
) -> Result<(), Diagnostic> {
    let result = op.result().ok_or_else(invariant)?;
    let address = c.value(b, result.id())?;
    let value = match c.size(result.ty())? {
        1 => b.ins().ireduce(types::I8, value),
        4 => value,
        _ => return Err(invariant()),
    };
    b.ins().store(MemFlagsData::new(), value, address, 0);
    Ok(())
}
