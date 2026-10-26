//! Same-seal invocation eligibility over existing scalar/aggregate/container execution roles.
use super::super::{
    Context, Writer,
    context::{Function, charge},
    emission_budget, internal,
};
use zryna_diagnostics::Diagnostic;
use zryna_ir::generic_v1::execution_v1::{VerifiedExecution, VerifiedParameter, raw};
use zryna_layout::TypeCategory;
pub(in crate::generic_execution_v1) struct Plan<'p, 'a> {
    owner: &'p VerifiedExecution<'a>,
}
pub(in crate::generic_execution_v1) const MANIFEST: &str =
    ",accounting:'baab442996b88580f2e2e07ec578e7cc8801fef54465dc6d6e80c8a073ca13b8'";
pub(in crate::generic_execution_v1) const LEDGER: &str = concat!(
    include_str!("accounting/ledger.js"),
    include_str!("accounting/vector.js"),
    include_str!("accounting/control.js")
);
impl<'p, 'a> Plan<'p, 'a> {
    pub(in crate::generic_execution_v1) fn derive(
        context: &Context<'p, 'a>,
    ) -> Result<Option<Self>, Diagnostic> {
        // All nine independently verified stored categories now have guarded child transport.
        // Retain the existing upfront type-row payment; this adds no type/graph walk.
        charge(&context.work, context.types.len())?;
        let mut string = false;
        charge(&context.work, context.functions.len())?;
        for function in &context.functions {
            if !effects(context, function, &mut string)? {
                return Ok(None);
            }
        }
        if !string {
            return Ok(None);
        }
        charge(&context.work, context.program.export_functions().len())?;
        for &entry in context.program.export_functions() {
            let function = context.functions.get(entry).ok_or_else(internal)?;
            charge(&context.work, function.parameters.len())?;
            for parameter in &function.parameters {
                let VerifiedParameter::Value(value) = parameter else { return Err(internal()) };
                scalar(context, value.ty())?;
            }
            scalar(context, function.view.result_type())?;
        }
        Ok(Some(Self { owner: context.program }))
    }
    pub(in crate::generic_execution_v1) fn bound(&self, context: &Context<'_, '_>) -> bool {
        std::ptr::eq(self.owner, context.program)
    }
}
fn effects(
    context: &Context<'_, '_>,
    function: &Function<'_, '_>,
    string: &mut bool,
) -> Result<bool, Diagnostic> {
    charge(&context.work, function.blocks.len())?;
    for block in &function.blocks {
        if block.weak_upgrade().is_some() {
            *string = true;
        }
        charge(&context.work, block.operations().len())?;
        for operation in block.operations() {
            if let Some(clone) = operation.clone_frontier() {
                // Cached all-variant failure fact: pure Copy aggregate glue needs no ledger.
                *string |= clone.frontier().may_fail();
            }
            if matches!(
                operation.kind(),
                raw::Operation::StringFromUtf8 { .. }
                    | raw::Operation::StringConcat { .. }
                    | raw::Operation::VecConstruct { .. }
                    | raw::Operation::VecPush { .. }
                    | raw::Operation::SharedConstruct { .. }
                    | raw::Operation::WeakDowngrade { .. }
            ) {
                *string = true;
            }
            if !matches!(
                operation.kind(),
                raw::Operation::Unit
                    | raw::Operation::BoolLiteral(_)
                    | raw::Operation::I32Literal(_)
                    | raw::Operation::ScalarBinary { .. }
                    | raw::Operation::I32Neg { .. }
                    | raw::Operation::ReadPlace { .. }
                    | raw::Operation::ReadLoan { .. }
                    | raw::Operation::InitializePlace { .. }
                    | raw::Operation::ReplacePlace { .. }
                    | raw::Operation::ReplaceLoan { .. }
                    | raw::Operation::DropPlace { .. }
                    | raw::Operation::Clone { .. }
                    | raw::Operation::BeginLoan { .. }
                    | raw::Operation::EndLoan { .. }
                    | raw::Operation::SourceCall { .. }
                    | raw::Operation::ClosedGenericCall { .. }
                    | raw::Operation::StructConstruct { .. }
                    | raw::Operation::ClosedEnumConstruct { .. }
                    | raw::Operation::FixedArrayConstruct { .. }
                    | raw::Operation::StringFromUtf8 { .. }
                    | raw::Operation::StringConcat { .. }
                    | raw::Operation::VecConstruct { .. }
                    | raw::Operation::VecPush { .. }
                    | raw::Operation::SharedConstruct { .. }
                    | raw::Operation::WeakDowngrade { .. }
                    | raw::Operation::IndexCopy { .. }
            ) {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
fn scalar(context: &Context<'_, '_>, ty: raw::ValueType) -> Result<(), Diagnostic> {
    let raw::ValueType::Stored(id) = ty else { return Err(internal()) };
    if matches!(context.ty(id)?.category(), TypeCategory::Bool | TypeCategory::I32) {
        Ok(())
    } else {
        Err(internal())
    }
}
pub(in crate::generic_execution_v1) fn argument(
    context: &Context<'_, '_>,
    writer: &mut Writer,
    preceding: bool,
) -> Result<(), Diagnostic> {
    if context.accounting.is_some() {
        if preceding {
            writer.text(",")?;
        }
        writer.text("$ctx")?;
    }
    Ok(())
}
pub(in crate::generic_execution_v1) fn declarations(
    context: &Context<'_, '_>,
    writer: &mut Writer,
) -> Result<(), Diagnostic> {
    if !context.accounting.as_ref().is_some_and(|plan| plan.bound(context)) {
        return Err(internal());
    }
    charge(&context.work, LEDGER.len())?;
    writer.trusted(LEDGER)
}
pub(in crate::generic_execution_v1) fn quota() -> Result<super::ordinary::quota::Quota, Diagnostic>
{
    Ok(super::ordinary::quota::Quota {
        allocations: usize::try_from(zryna_ownership_runtime_abi::MAX_ALLOCATION_OPERATIONS)
            .map_err(|_| emission_budget())?,
        transitions: usize::try_from(zryna_ownership_runtime_abi::MAX_STATUS_TRANSITIONS)
            .map_err(|_| emission_budget())?,
    })
}
