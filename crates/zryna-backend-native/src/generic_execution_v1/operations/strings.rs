//! Atomic canonical String concatenation; sources remain borrowed and failed outputs unpublished.
use super::{Frame, invariant, records};
use crate::generic_execution_v1::{body::failure, runtime};
use cranelift_frontend::FunctionBuilder;
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{VerifiedOperation, raw};
use zryna_ownership_runtime_abi::LogicalOperation;
pub(super) fn concat(
    c: &Frame<'_, '_, '_>,
    b: &mut FunctionBuilder<'_>,
    op: VerifiedOperation<'_, '_>,
) -> Result<(), Diagnostic> {
    let raw::Operation::StringConcat { left, right, .. } = op.kind() else {
        return Err(invariant());
    };
    let left = c.source(b, *left)?;
    let right = c.source(b, *right)?;
    let result = op.result().ok_or_else(invariant)?;
    let destination = c.value(b, result.id())?;
    records::zero(b, destination, c.size(result.ty())?)?;
    let status = runtime::call(
        c.p,
        c.runtime,
        b,
        LogicalOperation::StringConcat,
        &[left, right, destination],
    )?;
    failure(c, b, op.failure(), status)
}
