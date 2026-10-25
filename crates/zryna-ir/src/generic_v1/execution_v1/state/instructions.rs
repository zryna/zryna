//! Preparation snapshots precede operand commit and result publication.
use super::{Context, Failure, State, budget, plan, raw, reject, resource_failure};
mod containers;
mod indexed;
mod push;
mod reads;
mod replacement;
mod storage;

pub(super) fn execute(
    c: &Context<'_>,
    state: &mut State,
    bi: usize,
    ii: usize,
    i: &raw::Instruction,
    plan: &mut plan::Plan,
    spent: &mut usize,
) -> Result<(), Failure> {
    let mut debit = *spent;
    let work = &mut debit;
    super::access::instruction_credit(c, state, &i.operation, work)?;
    let mut tag = None;
    let mut publication_slot = None;
    let index = c.checked().sites[bi][ii];
    match &i.operation {
        raw::Operation::Unit | raw::Operation::BoolLiteral(_) | raw::Operation::I32Literal(_) => {}
        raw::Operation::ScalarBinary { left, right, .. } => {
            state.value(c, *left)?;
            state.value(c, *right)?;
        }
        raw::Operation::I32Neg { value } => state.value(c, *value)?,
        raw::Operation::ReadPlace { place, mode } => {
            (tag, publication_slot) = reads::place(c, state, *place, *mode)?;
        }
        raw::Operation::InitializePlace { .. }
        | raw::Operation::ReplacePlace { .. }
        | raw::Operation::DropPlace { .. } => storage::execute(c, state, index, i, plan, work)?,
        raw::Operation::Clone { .. } => {
            tag = clone_operation(c, state, (bi, ii), i, plan, work)?;
        }
        raw::Operation::StructConstruct { fields, .. } => {
            for value in fields {
                state.consume(c, *value)?;
            }
        }
        raw::Operation::FixedArrayConstruct { elements, .. } => {
            for value in elements {
                state.consume(c, *value)?;
            }
        }
        raw::Operation::ClosedEnumConstruct { ordinal, payload, .. } => {
            if let Some(id) = payload {
                state.consume(c, *id)?;
            }
            tag = Some(*ordinal);
        }
        raw::Operation::StringFromUtf8 { failure: id, .. } => {
            failure(c, state, *id, i.site_span, plan, work)?;
            plan.operations[index].failure = Some(*id);
        }
        raw::Operation::IndexCopy { base, index: value, failure: id } => {
            state.read(c, *base)?;
            state.value(c, *value)?;
            failure(c, state, *id, i.site_span, plan, work)?;
            plan.operations[index].failure = Some(*id);
        }
        raw::Operation::ClosedGenericCall { .. } | raw::Operation::SourceCall { .. } => {
            call(c, state, index, i, plan, work)?;
        }
        raw::Operation::BeginLoan { loan, .. } => {
            budget::charge(
                work,
                c.raw()
                    .loans
                    .len()
                    .checked_mul(c.raw().loans.len())
                    .ok_or_else(|| resource_failure("loan ancestor comparison overflow"))?,
            )?;
            indexed::begin(c, state, index, i, *loan, plan, work)?;
        }
        raw::Operation::EndLoan { loan } => state.end(c, *loan)?,
        raw::Operation::ReadLoan { loan } => {
            state.read_loan(c, *loan)?;
            tag = source_tag(c, state, raw::PlaceOrLoan::Loan(*loan))?;
        }
        raw::Operation::ReplaceLoan { .. } => replacement::execute(c, state, index, i, plan, work)?,
        raw::Operation::StringConcat { left, right, failure: id } => {
            state.read(c, *left)?;
            state.read(c, *right)?;
            failure(c, state, *id, i.site_span, plan, work)?;
            plan.operations[index].failure = Some(*id);
        }
        raw::Operation::VecConstruct { .. }
        | raw::Operation::SharedConstruct { .. }
        | raw::Operation::WeakDowngrade { .. } => {
            containers::execute(c, state, index, i, plan, work)?;
        }
        raw::Operation::VecPush { .. } => {
            push::execute(c, state, index, i, plan, work)?;
        }
    }
    if let Some(d) = &i.result {
        state.publish_at(c, d, tag, publication_slot)?;
    }
    *spent = debit;
    Ok(())
}
fn failure(
    c: &Context<'_>,
    state: &State,
    id: u32,
    span: zryna_source::UntrustedSpan,
    plan: &mut plan::Plan,
    work: &mut usize,
) -> Result<(), Failure> {
    plan.claim(c.program, c.function, id, &state.actions(c, work)?, Some(span), work)
}
fn source_tag(
    c: &Context<'_>,
    state: &State,
    source: raw::PlaceOrLoan,
) -> Result<Option<u32>, Failure> {
    let place = match source {
        raw::PlaceOrLoan::Place(id) => Some(id),
        raw::PlaceOrLoan::Loan(id) => super::access::referent_place(c, id)?,
    };
    Ok(place.and_then(|id| state.tag(c, id)))
}

fn clone_operation(
    c: &Context<'_>,
    state: &State,
    site: (usize, usize),
    i: &raw::Instruction,
    plan: &mut plan::Plan,
    work: &mut usize,
) -> Result<Option<u32>, Failure> {
    let raw::Operation::Clone { source, outer_failure, prefix_failure } = &i.operation else {
        return Err(reject("clone preparation requires exact Clone opcode"));
    };
    let (bi, ii) = site;
    let index = c.checked().sites[bi][ii];

    state.read(c, *source)?;
    let tag = source_tag(c, state, *source)?;
    let result = i.result.as_ref().ok_or_else(|| reject("clone result absent"))?;
    let destination = c.checked().places.owners[result.id as usize];
    if destination.is_some_and(|id| state.initialized[id as usize]) {
        return Err(reject("clone destination is already initialized"));
    }
    if let Some(outer) = outer_failure {
        failure(c, state, *outer, i.site_span, plan, work)?;
        let mut actions = state.actions(c, work)?;
        let destination =
            destination.ok_or_else(|| reject("fallible clone has no exact pending destination"))?;
        budget::charge(
            work,
            actions
                .len()
                .checked_add(1)
                .ok_or_else(|| resource_failure("clone prefix action overflow"))?,
        )?;
        actions.try_reserve(1).map_err(|_| Failure::AllocationFailure)?;
        actions.insert(
            0,
            raw::CleanupAction::DropInitializedPrefix {
                destination,
                preparation_site: raw::PreparationSite {
                    function: u32::try_from(c.function).map_err(|_| Failure::InternalFailure)?,
                    block: u32::try_from(bi).map_err(|_| Failure::InternalFailure)?,
                    instruction: u32::try_from(ii).map_err(|_| Failure::InternalFailure)?,
                },
            },
        );
        plan.claim(
            c.program,
            c.function,
            prefix_failure.ok_or_else(|| reject("clone prefix plan absent"))?,
            &actions,
            Some(i.site_span),
            work,
        )?;
    }
    plan.operations[index] = plan::OperationPlan {
        destination,
        failure: *outer_failure,
        prefix: *prefix_failure,
        drops: None,
    };
    Ok(tag)
}

fn call(
    c: &Context<'_>,
    state: &mut State,
    index: usize,
    i: &raw::Instruction,
    plan: &mut plan::Plan,
    work: &mut usize,
) -> Result<(), Failure> {
    let (raw::Operation::ClosedGenericCall { arguments, failure: id, .. }
    | raw::Operation::SourceCall { arguments, failure: id, .. }) = &i.operation
    else {
        return Err(reject("call state requires exact call operation"));
    };

    budget::charge(
        work,
        arguments
            .len()
            .checked_mul(arguments.len())
            .and_then(|n| n.checked_add(arguments.len().checked_mul(c.raw().loans.len())?))
            .ok_or_else(|| resource_failure("call alias comparison overflow"))?,
    )?;
    state.call_loans(c, arguments)?;
    for argument in arguments {
        if let raw::CallArgument::Value(id) = argument {
            state.consume(c, *id)?;
        }
    }
    // A callee may mutate an exclusive referent before either returned outcome.
    // Preserve by-value consumption order and forget only that referent's facts
    // before checking failure cleanup or authorizing the success continuation.
    budget::charge(work, arguments.len())?;
    for argument in arguments {
        if let raw::CallArgument::Loan(loan) = argument
            && c.raw().loans[*loan as usize].access == raw::Access::Exclusive
        {
            super::access::invalidate_call(c, state, *loan, work)?;
        }
    }
    if let Some(id) = id {
        failure(c, state, *id, i.site_span, plan, work)?;
        plan.operations[index].failure = Some(*id);
    }
    Ok(())
}
