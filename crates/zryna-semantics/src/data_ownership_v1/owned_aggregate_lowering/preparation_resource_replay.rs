use super::super::super::Errors;
use super::super::super::owned_constructor_plan::ConstructorKind;
use super::super::super::owned_lowering_resources::{CleanupUsage, OwnedCleanupPlanContext};
use super::super::preparation_plan::{Operation, PreparationPlan};
use super::super::preparation_state::Checkpoint;
use super::super::resource_decisions::AggregateUsage;

#[path = "preparation_vec_resources.rs"]
pub(super) mod vec_resources;

fn usage(at: Checkpoint) -> AggregateUsage {
    AggregateUsage {
        values: at.counts[0],
        places: at.counts[1],
        transitions: at.counts[2],
        operands: at.counts[3],
        held_operands: at.held[0],
        held_transitions: at.held[1],
        held_values: at.held[2],
        held_places: at.held[3],
    }
}

fn cleanup(at: Checkpoint) -> CleanupUsage {
    CleanupUsage {
        plans: at.counts[4],
        actions: at.counts[5],
        reserved_plans: at.held_cleanup[0],
        reserved_actions: at.held_cleanup[1],
    }
}

pub(super) fn clone_capacity(
    before: Checkpoint,
    aggregate: bool,
    at: zryna_source::Span,
    errors: &mut Errors<'_>,
) -> Option<()> {
    let resources = usage(before);
    super::super::clone_decisions::CloneUsage {
        values: resources.values.saturating_add(resources.held_values),
        places: resources.places.saturating_add(resources.held_places),
        transitions: resources.transitions,
        reserved_transitions: resources.held_transitions,
        cleanup_plans: before.counts[4].saturating_add(before.held_cleanup[0]),
        cleanup_actions: before.counts[5].saturating_add(before.held_cleanup[1]),
        pending: before.pending,
    }
    .validate(aggregate, at, errors)
}

fn ordinary_step(
    step: &super::super::preparation_plan::Step<'_>,
    before: Checkpoint,
    (constructor_depth, vector_parent, released_vec): (usize, bool, bool),
    calls: &[(usize, usize, super::super::preparation_plan::CallKind)],
    released_call: Option<super::super::preparation_plan::CallKind>,
    errors: &mut Errors<'_>,
) -> Option<()> {
    match &step.operation {
        Operation::Cleanup { actions, prefix: None, .. } => {
            let vector = released_vec || vector_parent;
            let call = released_call.or_else(|| {
                calls
                    .last()
                    .filter(|(depth, _, _)| *depth == constructor_depth)
                    .map(|(_, _, kind)| *kind)
            });
            if (vector || call.is_some())
                && !vec_resources::capacity(before).transitions(1, step.at, errors)
            {
                return None;
            }
            if !cleanup(before).validate_reverse(
                *actions,
                if call == Some(super::super::preparation_plan::CallKind::String) {
                    OwnedCleanupPlanContext::String
                } else if vector || call.is_some() {
                    OwnedCleanupPlanContext::Vec
                } else {
                    OwnedCleanupPlanContext::Aggregate
                },
                step.at,
                errors,
            ) {
                return None;
            }
        }
        Operation::Leaf(_) => {
            let valid = if vector_parent
                || calls.last().is_some_and(|(depth, _, _)| *depth == constructor_depth)
            {
                vec_resources::emit(before, step.ty, step.at, errors)
            } else {
                usage(before).emit(step.ty, step.at, errors)
            };
            if !valid {
                return None;
            }
        }
        _ => unreachable!("ordinary resource operation"),
    }
    Some(())
}

// A successful semantic summary already owns every type, owner effect and exact cleanup
// demand. This pass validates only recorded costs; it never visits source or changes owners.
pub(super) fn validate(
    plan: &mut PreparationPlan<'_>,
    layouts: &zryna_layout::VerifiedLayouts,
    errors: &mut Errors<'_>,
) -> Option<()> {
    let mut before = plan.start;
    let mut held = before.held_cleanup;
    let mut frames = Vec::new();
    let mut released_vec = false;
    let mut calls = Vec::new();
    let mut released_call = None;
    for index in 0..plan.steps.len() {
        let step = &plan.steps[index];
        let resources = usage(before);
        match &step.operation {
            Operation::CallEnter { signature, .. } => {
                let (actions, reserved) =
                    super::call_resources::enter(plan, index, before, errors)?;
                held = reserved;
                calls.push((frames.len(), actions, signature.kind));
            }
            Operation::CallRelease => {
                let (depth, actions, kind) = calls.pop()?;
                assert_eq!(depth, frames.len(), "call resource frame depth");
                held = CleanupUsage::release(held, actions);
                released_call = Some(kind);
            }
            Operation::CallCommit { .. } => {
                if !vec_resources::emit(before, step.ty, step.at, errors) {
                    return None;
                }
                released_call = None;
            }
            Operation::Enter { arity, kind, .. } => {
                let actions = if *kind == ConstructorKind::Vec {
                    let (actions, reserved) =
                        vec_resources::enter(plan, index, before, layouts, errors)?;
                    held = reserved;
                    if !resources.operands(*arity, step.at, errors) {
                        return None;
                    }
                    Some(actions)
                } else {
                    if !resources.constructor(step.ty, *arity, step.at, errors) {
                        return None;
                    }
                    None
                };
                frames.push(actions);
            }
            Operation::Release => {
                if let Some(actions) = frames.pop()? {
                    held = CleanupUsage::release(held, actions);
                    released_vec = true;
                }
            }
            Operation::Prefix { .. } => {
                if !super::super::projection_topology::projection_capacity(
                    resources.places.saturating_add(resources.held_places),
                    step.at,
                    errors,
                ) {
                    return None;
                }
            }
            Operation::CloneCapacity { aggregate } => {
                clone_capacity(before, *aggregate, step.at, errors)?;
            }
            Operation::CallTransfer { .. }
            | Operation::StringEnter { .. }
            | Operation::StringRead(_)
            | Operation::StringExit
            | Operation::Cleanup { prefix: Some(_), .. } => {}
            Operation::Cleanup { prefix: None, .. } | Operation::Leaf(_) => {
                ordinary_step(
                    step,
                    before,
                    (frames.len(), frames.last().is_some_and(Option::is_some), released_vec),
                    &calls,
                    released_call,
                    errors,
                )?;
            }
            Operation::VecCommit { values, .. } => {
                if !resources.operands(values.len(), step.at, errors)
                    || !vec_resources::emit(before, step.ty, step.at, errors)
                {
                    return None;
                }
                released_vec = false;
            }
            Operation::Commit { values, .. } => {
                if !resources.operands(values.len(), step.at, errors)
                    || !resources.emit(step.ty, step.at, errors)
                {
                    return None;
                }
            }
        }
        let step = &mut plan.steps[index];
        step.after.held_cleanup = held;
        before = step.after;
    }
    finish(plan, frames.is_empty(), calls.is_empty() && released_call.is_none(), held);
    Some(())
}

fn finish(
    plan: &mut PreparationPlan<'_>,
    constructors_clear: bool,
    calls_clear: bool,
    held: [usize; 2],
) {
    assert!(constructors_clear, "summarized constructor frames balance");
    assert!(calls_clear, "summarized call frames balance");
    assert_eq!(held, plan.start.held_cleanup, "summary releases only its own cleanup credits");
    plan.facts.held_cleanup = held;
}
