//! Resource-effect certificate for cycles; never a runtime iteration/fuel bound.
use super::{Context, Diagnostic, Function, Quota, charge, emission_budget, limit, raw};
mod graph;
mod types;

pub(super) fn prove(context: &Context<'_, '_>, entry: usize) -> Result<Option<Quota>, Diagnostic> {
    let (graph, cyclic) = graph::Graph::build(context, entry)?;
    if !cyclic {
        return Ok(None);
    }
    graph.check_depth(context, entry)?;
    let mut types = types::Types::new(context)?;
    charge(&context.work, graph.functions.len())?;
    for &index in &graph.functions {
        let function = context.functions.get(index).ok_or_else(super::internal)?;
        charge(
            &context.work,
            function
                .values
                .len()
                .checked_add(function.places.len())
                .and_then(|n| n.checked_add(function.loans.len()))
                .ok_or_else(emission_budget)?,
        )?;
        types.value(context, function.view.result_type())?;
        for &ty in &function.values {
            types.value(context, ty)?;
        }
        for place in &function.places {
            types.request(context, place.ty())?;
        }
        for loan in &function.loans {
            types.request(context, loan.referent())?;
        }
        effects(context, function)?;
    }
    types.check(context)?;
    let allocations = limit(zryna_ownership_runtime_abi::MAX_ALLOCATION_OPERATIONS)?;
    let transitions = limit(zryna_ownership_runtime_abi::MAX_STATUS_TRANSITIONS)?;
    // Every resource is a nonempty String: each acquisition attempts physical allocation,
    // and each successful acquisition has one eventual release. A terminal denied request
    // contributes at most one further status; no source catch resumes after controlled failure.
    if allocations.checked_mul(2).and_then(|n| n.checked_add(1)).is_none_or(|n| n > transitions) {
        return Err(emission_budget());
    }
    Ok(Some(Quota { allocations, transitions }))
}

fn effects(context: &Context<'_, '_>, function: &Function<'_, '_>) -> Result<(), Diagnostic> {
    charge(&context.work, function.blocks.len())?;
    for block in &function.blocks {
        if block.weak_upgrade().is_some() {
            return Err(emission_budget());
        }
        let operations = block.operations();
        charge(&context.work, operations.len())?;
        for operation in operations {
            match operation.kind() {
                raw::Operation::StringFromUtf8 { bytes, .. } => {
                    if bytes.is_empty() {
                        return Err(emission_budget());
                    }
                }
                raw::Operation::VecConstruct { .. }
                | raw::Operation::VecPush { .. }
                | raw::Operation::SharedConstruct { .. }
                | raw::Operation::WeakDowngrade { .. } => return Err(emission_budget()),
                raw::Operation::Unit
                | raw::Operation::BoolLiteral(_)
                | raw::Operation::I32Literal(_)
                | raw::Operation::ScalarBinary { .. }
                | raw::Operation::I32Neg { .. }
                | raw::Operation::ReadPlace { .. }
                | raw::Operation::InitializePlace { .. }
                | raw::Operation::ReplacePlace { .. }
                | raw::Operation::DropPlace { .. }
                | raw::Operation::Clone { .. }
                | raw::Operation::ClosedGenericCall { .. }
                | raw::Operation::SourceCall { .. }
                | raw::Operation::StructConstruct { .. }
                | raw::Operation::ClosedEnumConstruct { .. }
                | raw::Operation::FixedArrayConstruct { .. }
                | raw::Operation::StringConcat { .. }
                | raw::Operation::BeginLoan { .. }
                | raw::Operation::EndLoan { .. }
                | raw::Operation::ReadLoan { .. }
                | raw::Operation::ReplaceLoan { .. }
                | raw::Operation::IndexCopy { .. } => {}
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
