//! Same-retained-runtime control8 call routing; no new ABI declaration or status.
use super::{
    Diagnostic, FuncRef, FunctionBuilder, LogicalOperation, NAMES, Value, VerifiedProgram,
    invariant,
};
use zryna_native_mir::generic_execution_v1::ControlEffect;
fn reference(
    p: &VerifiedProgram<'_, '_>,
    refs: &[FuncRef],
    effect: ControlEffect<'_, '_>,
) -> Result<FuncRef, Diagnostic> {
    if !effect.belongs_to(p.program()) {
        return Err(invariant());
    }
    let index = NAMES.len()
        + p.containers()
            .ok_or_else(invariant)?
            .operations()
            .iter()
            .position(|op| *op == effect.operation())
            .ok_or_else(invariant)?;
    refs.get(index).copied().ok_or_else(invariant)
}
pub(in crate::generic_execution_v1) fn control(
    p: &VerifiedProgram<'_, '_>,
    refs: &[FuncRef],
    b: &mut FunctionBuilder<'_>,
    invocation: Value,
    effect: ControlEffect<'_, '_>,
    args: &[Value],
) -> Result<Value, Diagnostic> {
    let function = reference(p, refs, effect)?;
    super::super::accounting::control::call(b, invocation, function, effect, args)
}
pub(in crate::generic_execution_v1) fn control_release(
    p: &VerifiedProgram<'_, '_>,
    refs: &[FuncRef],
    b: &mut FunctionBuilder<'_>,
    invocation: Value,
    effect: ControlEffect<'_, '_>,
    args: &[Value],
) -> Result<(), Diagnostic> {
    let function = reference(p, refs, effect)?;
    super::super::accounting::control::release(b, invocation, function, effect, args)
}
pub(in crate::generic_execution_v1) fn control_operation(
    p: &VerifiedProgram<'_, '_>,
    refs: &[FuncRef],
    function: FuncRef,
) -> Option<LogicalOperation> {
    use LogicalOperation as O;
    let plan = p.containers()?;
    let index =
        refs.iter().position(|reference| *reference == function)?.checked_sub(NAMES.len())?;
    let operation = *plan.operations().get(index)?;
    matches!(
        operation,
        O::Allocate
            | O::StrongClone
            | O::WeakDowngrade
            | O::WeakClone
            | O::WeakUpgrade
            | O::StrongReleaseBegin
            | O::StrongReleaseFinish
            | O::WeakRelease
    )
    .then_some(operation)
}
