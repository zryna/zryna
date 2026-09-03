use std::collections::BTreeMap;

#[cfg(test)]
use super::global_resource_limits::resource_budget_violation;
use super::owner_state::OwnerState;
use super::type_model::{Binding, Ty};
#[cfg(test)]
use zryna_ir::data_ownership_v1 as ir;
use zryna_source::UntrustedSpan;
use zryna_syntax::v4::{self as syntax, RawExpressionKind};

#[cfg(test)]
pub(super) fn owned_call_cleanup_budget_violation(
    cleanup_plans: usize,
    cleanup_actions: usize,
    pending: usize,
    transferred: usize,
) -> bool {
    cleanup_plans >= ir::MAX_CLEANUP_PLANS_PER_FUNCTION
        || pending.checked_sub(transferred).is_none_or(|survivors| {
            resource_budget_violation(cleanup_actions, survivors, ir::MAX_DROP_ACTIONS_PER_FUNCTION)
        })
}

pub(super) const fn vec_push_target_invalid(mutable: bool, available: bool) -> bool {
    !mutable || !available
}

pub(super) const fn cleanup_actions_after_preparation(
    pending: usize,
    creates_owner: bool,
) -> usize {
    pending.saturating_add(creates_owner as usize)
}

pub(super) const fn cleanup_actions_after_transfer(
    pending: usize,
    transfers_existing: bool,
) -> usize {
    pending.saturating_sub(transfers_existing as usize)
}

pub(super) const fn cleanup_actions_after_additions(
    pending: usize,
    additional_owners: usize,
) -> usize {
    pending.saturating_add(additional_owners)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct OwnedStringPreparationEstimate {
    pub(super) end_pending: usize,
    pub(super) peak_pending: usize,
    pub(super) cleanup_plans: usize,
    pub(super) cleanup_actions: usize,
    pub(super) values: usize,
    pub(super) places: usize,
    pub(super) transitions: usize,
    pub(super) transfers_existing: bool,
    pub(super) root_cleanup_actions: Option<usize>,
}

#[derive(Clone, Copy)]
pub(super) enum OwnedStringEstimateContext {
    Value,
    Read,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum OwnedStringEstimateError {
    Unsupported,
    Unavailable(UntrustedSpan),
    Overflow,
}

pub(super) enum OwnedStringEstimateOutcome {
    Estimated(OwnedStringPreparationEstimate),
    Unsupported,
}

pub(super) fn add_estimate_counts(
    left: OwnedStringPreparationEstimate,
    right: OwnedStringPreparationEstimate,
) -> Result<OwnedStringPreparationEstimate, OwnedStringEstimateError> {
    Ok(OwnedStringPreparationEstimate {
        end_pending: right.end_pending,
        peak_pending: left.peak_pending.max(right.peak_pending),
        cleanup_plans: left
            .cleanup_plans
            .checked_add(right.cleanup_plans)
            .ok_or(OwnedStringEstimateError::Overflow)?,
        cleanup_actions: left
            .cleanup_actions
            .checked_add(right.cleanup_actions)
            .ok_or(OwnedStringEstimateError::Overflow)?,
        values: left.values.checked_add(right.values).ok_or(OwnedStringEstimateError::Overflow)?,
        places: left.places.checked_add(right.places).ok_or(OwnedStringEstimateError::Overflow)?,
        transitions: left
            .transitions
            .checked_add(right.transitions)
            .ok_or(OwnedStringEstimateError::Overflow)?,
        transfers_existing: left.transfers_existing || right.transfers_existing,
        root_cleanup_actions: right.root_cleanup_actions,
    })
}

fn estimate_owned_string_leaf(
    expression: &syntax::RawExpressionSyntax,
    bindings: &BTreeMap<String, Binding>,
    owners: &OwnerState,
    string_ty: Ty,
    pending: usize,
    context: OwnedStringEstimateContext,
) -> Option<Result<OwnedStringPreparationEstimate, OwnedStringEstimateError>> {
    let empty = || OwnedStringPreparationEstimate {
        end_pending: pending,
        peak_pending: pending,
        cleanup_plans: 0,
        cleanup_actions: 0,
        values: 0,
        places: 0,
        transitions: 0,
        transfers_existing: false,
        root_cleanup_actions: None,
    };
    match &expression.kind {
        RawExpressionKind::Reference { name } => {
            let binding = bindings.get(&name.text).filter(|binding| binding.ty == string_ty)?;
            if !owners.contains(binding.place) {
                return Some(Err(OwnedStringEstimateError::Unavailable(name.span)));
            }
            if matches!(context, OwnedStringEstimateContext::Read) {
                return Some(Ok(empty()));
            }
            Some(
                pending
                    .checked_add(1)
                    .map(|peak_pending| OwnedStringPreparationEstimate {
                        transfers_existing: true,
                        values: 1,
                        places: 1,
                        transitions: 1,
                        peak_pending,
                        ..empty()
                    })
                    .ok_or(OwnedStringEstimateError::Overflow),
            )
        }
        RawExpressionKind::StringLiteral { .. } => Some(
            pending
                .checked_add(1)
                .map(|end| OwnedStringPreparationEstimate {
                    end_pending: end,
                    peak_pending: end,
                    cleanup_plans: 1,
                    cleanup_actions: pending,
                    values: 1,
                    places: 1,
                    transitions: 1,
                    transfers_existing: false,
                    root_cleanup_actions: Some(pending),
                })
                .ok_or(OwnedStringEstimateError::Overflow),
        ),
        _ => None,
    }
}

pub(super) fn estimate_owned_string_expression(
    function: &syntax::RawFunctionSyntax,
    bindings: &BTreeMap<String, Binding>,
    owners: &OwnerState,
    string_ty: Ty,
    id: u32,
    pending: usize,
    context: OwnedStringEstimateContext,
) -> Result<OwnedStringPreparationEstimate, OwnedStringEstimateError> {
    let expression = usize::try_from(id)
        .ok()
        .and_then(|index| function.body.expressions.get(index))
        .ok_or(OwnedStringEstimateError::Unsupported)?;
    if let Some(estimate) =
        estimate_owned_string_leaf(expression, bindings, owners, string_ty, pending, context)
    {
        return estimate;
    }
    let empty = || OwnedStringPreparationEstimate {
        end_pending: pending,
        peak_pending: pending,
        cleanup_plans: 0,
        cleanup_actions: 0,
        values: 0,
        places: 0,
        transitions: 0,
        transfers_existing: false,
        root_cleanup_actions: None,
    };
    match &expression.kind {
        RawExpressionKind::Clone { value, .. } => {
            let child = estimate_owned_string_expression(
                function,
                bindings,
                owners,
                string_ty,
                *value,
                pending,
                OwnedStringEstimateContext::Read,
            )?;
            let end = child.end_pending.checked_add(1).ok_or(OwnedStringEstimateError::Overflow)?;
            Ok(OwnedStringPreparationEstimate {
                end_pending: end,
                peak_pending: child.peak_pending.max(end),
                cleanup_plans: child
                    .cleanup_plans
                    .checked_add(1)
                    .ok_or(OwnedStringEstimateError::Overflow)?,
                cleanup_actions: child
                    .cleanup_actions
                    .checked_add(child.end_pending)
                    .ok_or(OwnedStringEstimateError::Overflow)?,
                values: child.values.checked_add(1).ok_or(OwnedStringEstimateError::Overflow)?,
                places: child.places.checked_add(1).ok_or(OwnedStringEstimateError::Overflow)?,
                transitions: child
                    .transitions
                    .checked_add(1)
                    .ok_or(OwnedStringEstimateError::Overflow)?,
                transfers_existing: child.transfers_existing,
                root_cleanup_actions: Some(child.end_pending),
            })
        }
        RawExpressionKind::Call { callee, arguments, .. } if callee.text == "concat" => {
            let mut total = empty();
            for argument in arguments {
                let child = estimate_owned_string_expression(
                    function,
                    bindings,
                    owners,
                    string_ty,
                    *argument,
                    total.end_pending,
                    OwnedStringEstimateContext::Read,
                )?;
                total = add_estimate_counts(total, child)?;
            }
            let cleanup_at_root = total.end_pending;
            let end = cleanup_at_root.checked_add(1).ok_or(OwnedStringEstimateError::Overflow)?;
            Ok(OwnedStringPreparationEstimate {
                end_pending: end,
                peak_pending: total.peak_pending.max(end),
                cleanup_plans: total
                    .cleanup_plans
                    .checked_add(1)
                    .ok_or(OwnedStringEstimateError::Overflow)?,
                cleanup_actions: total
                    .cleanup_actions
                    .checked_add(cleanup_at_root)
                    .ok_or(OwnedStringEstimateError::Overflow)?,
                values: total.values.checked_add(1).ok_or(OwnedStringEstimateError::Overflow)?,
                places: total.places.checked_add(1).ok_or(OwnedStringEstimateError::Overflow)?,
                transitions: total
                    .transitions
                    .checked_add(1)
                    .ok_or(OwnedStringEstimateError::Overflow)?,
                transfers_existing: total.transfers_existing,
                root_cleanup_actions: Some(cleanup_at_root),
            })
        }
        RawExpressionKind::Call { arguments, .. } => estimate_owned_string_call_arguments(
            function, bindings, owners, string_ty, arguments, pending,
        ),
        _ => Err(OwnedStringEstimateError::Unsupported),
    }
}

pub(super) fn estimate_owned_string_call_arguments(
    function: &syntax::RawFunctionSyntax,
    bindings: &BTreeMap<String, Binding>,
    owners: &OwnerState,
    string_ty: Ty,
    arguments: &[u32],
    pending: usize,
) -> Result<OwnedStringPreparationEstimate, OwnedStringEstimateError> {
    let mut total = OwnedStringPreparationEstimate {
        end_pending: pending,
        peak_pending: pending,
        cleanup_plans: 0,
        cleanup_actions: 0,
        values: 0,
        places: 0,
        transitions: 0,
        transfers_existing: false,
        root_cleanup_actions: None,
    };
    for argument in arguments {
        let child = estimate_owned_string_expression(
            function,
            bindings,
            owners,
            string_ty,
            *argument,
            total.end_pending,
            OwnedStringEstimateContext::Value,
        )?;
        total = add_estimate_counts(total, child)?;
        total.end_pending =
            total.end_pending.checked_sub(1).ok_or(OwnedStringEstimateError::Overflow)?;
    }
    let cleanup_at_root = total.end_pending;
    let end = cleanup_at_root.checked_add(1).ok_or(OwnedStringEstimateError::Overflow)?;
    Ok(OwnedStringPreparationEstimate {
        end_pending: end,
        peak_pending: total.peak_pending.max(end),
        cleanup_plans: total
            .cleanup_plans
            .checked_add(1)
            .ok_or(OwnedStringEstimateError::Overflow)?,
        cleanup_actions: total
            .cleanup_actions
            .checked_add(cleanup_at_root)
            .ok_or(OwnedStringEstimateError::Overflow)?,
        values: total.values.checked_add(1).ok_or(OwnedStringEstimateError::Overflow)?,
        places: total.places.checked_add(1).ok_or(OwnedStringEstimateError::Overflow)?,
        transitions: total.transitions.checked_add(1).ok_or(OwnedStringEstimateError::Overflow)?,
        transfers_existing: total.transfers_existing,
        root_cleanup_actions: Some(cleanup_at_root),
    })
}

#[derive(Clone, Copy)]
pub(super) struct VecPreparationEstimate {
    pub(super) end_pending: usize,
    pub(super) resources: OwnedStringPreparationEstimate,
}

pub(super) fn empty_owned_string_estimate(pending: usize) -> OwnedStringPreparationEstimate {
    OwnedStringPreparationEstimate {
        end_pending: pending,
        peak_pending: pending,
        cleanup_plans: 0,
        cleanup_actions: 0,
        values: 0,
        places: 0,
        transitions: 0,
        transfers_existing: false,
        root_cleanup_actions: None,
    }
}
