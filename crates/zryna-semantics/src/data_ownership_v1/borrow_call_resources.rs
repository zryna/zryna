use std::collections::VecDeque;

use super::diagnostics::Errors;
use super::function_catalog::{FunctionCatalog, FunctionResolution};
use super::type_model::{RootBorrowBudgetLimit, RootBorrowResources};
use super::{SemanticInput, span};
use zryna_ir::data_ownership_v1 as ir;
use zryna_syntax::v4::RawExpressionKind;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BorrowCallProgramBudgetLimit {
    CallEdges,
    CallDepth,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BorrowCallPreflightError {
    Overflow,
    Limit(RootBorrowBudgetLimit),
}

pub(super) fn checked_add_resources(
    current: RootBorrowResources,
    additional: RootBorrowResources,
) -> Result<RootBorrowResources, BorrowCallPreflightError> {
    let total = RootBorrowResources {
        values: current
            .values
            .checked_add(additional.values)
            .ok_or(BorrowCallPreflightError::Overflow)?,
        places: current
            .places
            .checked_add(additional.places)
            .ok_or(BorrowCallPreflightError::Overflow)?,
        transitions: current
            .transitions
            .checked_add(additional.transitions)
            .ok_or(BorrowCallPreflightError::Overflow)?,
        blocks: current
            .blocks
            .checked_add(additional.blocks)
            .ok_or(BorrowCallPreflightError::Overflow)?,
        edges: current
            .edges
            .checked_add(additional.edges)
            .ok_or(BorrowCallPreflightError::Overflow)?,
        active_peak: current
            .active_peak
            .checked_add(additional.active_peak)
            .ok_or(BorrowCallPreflightError::Overflow)?,
        cleanup_plans: current
            .cleanup_plans
            .checked_add(additional.cleanup_plans)
            .ok_or(BorrowCallPreflightError::Overflow)?,
    };
    let limit = if total.values > ir::MAX_VALUES_PER_FUNCTION {
        Some(RootBorrowBudgetLimit::Values)
    } else if total.places > ir::MAX_PLACES_PER_FUNCTION {
        Some(RootBorrowBudgetLimit::Places)
    } else if total.transitions > ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION {
        Some(RootBorrowBudgetLimit::Transitions)
    } else if total.blocks > ir::MAX_BLOCKS_PER_FUNCTION {
        Some(RootBorrowBudgetLimit::Blocks)
    } else if total.edges > ir::MAX_CFG_EDGES_PER_FUNCTION {
        Some(RootBorrowBudgetLimit::Edges)
    } else if total.active_peak > ir::MAX_ACTIVE_BORROWS_PER_FUNCTION {
        Some(RootBorrowBudgetLimit::ActiveBorrows)
    } else if total.cleanup_plans > ir::MAX_CLEANUP_PLANS_PER_FUNCTION {
        Some(RootBorrowBudgetLimit::CleanupPlans)
    } else {
        None
    };
    limit.map_or(Ok(total), |limit| Err(BorrowCallPreflightError::Limit(limit)))
}

pub(super) fn checked_call_delta(
    nested: RootBorrowResources,
    result_place: bool,
) -> Option<RootBorrowResources> {
    Some(RootBorrowResources {
        values: nested.values.checked_add(1)?,
        places: nested.places.checked_add(usize::from(result_place))?,
        transitions: nested.transitions.checked_add(1)?,
        blocks: nested.blocks,
        edges: nested.edges,
        active_peak: nested.active_peak,
        cleanup_plans: nested.cleanup_plans.checked_add(1)?,
    })
}

fn add(left: usize, right: usize) -> Option<usize> {
    left.checked_add(right)
}

fn multiply(left: usize, right: usize) -> Option<usize> {
    left.checked_mul(right)
}

pub(super) fn checked_merge_estimates(
    left: RootBorrowResources,
    right: RootBorrowResources,
) -> Option<RootBorrowResources> {
    Some(RootBorrowResources {
        values: left.values.checked_add(right.values)?,
        places: left.places.checked_add(right.places)?,
        transitions: left.transitions.checked_add(right.transitions)?,
        blocks: left.blocks.checked_add(right.blocks)?,
        edges: left.edges.checked_add(right.edges)?,
        active_peak: left.active_peak.max(right.active_peak),
        cleanup_plans: left.cleanup_plans.checked_add(right.cleanup_plans)?,
    })
}

pub(super) fn one_value_transition() -> RootBorrowResources {
    RootBorrowResources { values: 1, transitions: 1, ..RootBorrowResources::default() }
}

pub(super) fn checked_straight_borrow_call_resources(
    aliases: usize,
    reads: usize,
    writes: usize,
    calls: usize,
    call_values: usize,
) -> Option<RootBorrowResources> {
    let values = add(add(add(add(reads, writes)?, call_values)?, calls)?, 2)?;
    let places = add(add(reads, calls)?, 1)?;
    let transitions = add(
        add(
            add(add(multiply(aliases, 2)?, multiply(reads, 2)?)?, multiply(writes, 2)?)?,
            call_values,
        )?,
        add(multiply(calls, 2)?, 3)?,
    )?;
    Some(RootBorrowResources {
        values,
        places,
        transitions,
        blocks: 1,
        edges: 0,
        active_peak: aliases,
        cleanup_plans: add(calls, 1)?,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn checked_projected_borrow_call_resources(
    initializer_values: usize,
    aliases: usize,
    reads: usize,
    writes: usize,
    write_values: usize,
    projection_places: usize,
    calls: usize,
    call_values: usize,
) -> Option<RootBorrowResources> {
    let values = add(
        add(add(add(initializer_values, reads)?, write_values)?, 1)?,
        add(call_values, calls)?,
    )?;
    let places = add(add(reads, projection_places)?, add(calls, 1)?)?;
    let transitions = add(
        add(
            add(
                add(add(add(initializer_values, 1)?, multiply(aliases, 2)?)?, multiply(reads, 2)?)?,
                write_values,
            )?,
            add(writes, 1)?,
        )?,
        add(call_values, multiply(calls, 2)?)?,
    )?;
    Some(RootBorrowResources {
        values,
        places,
        transitions,
        blocks: 1,
        edges: 0,
        active_peak: aliases,
        cleanup_plans: add(calls, 1)?,
    })
}

pub(super) fn borrow_call_program_budget_violation(
    call_edges: usize,
    call_depth: usize,
) -> Option<BorrowCallProgramBudgetLimit> {
    if call_edges > ir::MAX_CALL_EDGES {
        Some(BorrowCallProgramBudgetLimit::CallEdges)
    } else if call_depth > ir::MAX_STATIC_CALL_DEPTH {
        Some(BorrowCallProgramBudgetLimit::CallDepth)
    } else {
        None
    }
}

#[allow(clippy::too_many_lines)]
pub(super) fn preflight_program_borrow_calls(
    input: SemanticInput<'_>,
    catalog: &FunctionCatalog,
    errors: &mut Errors<'_>,
) {
    let offsets = catalog
        .modules
        .iter()
        .scan(0_usize, |next, module| {
            let offset = *next;
            *next = next.checked_add(module.len())?;
            Some(offset)
        })
        .collect::<Vec<_>>();
    if offsets.len() != catalog.modules.len() {
        errors.global(
            "ZRYNA-M3201",
            "borrow-call graph sizing overflows checked arithmetic",
            "reduce private functions with borrow-parameter calls",
        );
        return;
    }
    let Some(function_count) =
        catalog.modules.iter().try_fold(0_usize, |count, module| count.checked_add(module.len()))
    else {
        errors.global(
            "ZRYNA-M3201",
            "borrow-call graph sizing overflows checked arithmetic",
            "reduce private functions with borrow-parameter calls",
        );
        return;
    };
    let mut graph = vec![Vec::<(usize, zryna_source::Span)>::new(); function_count];
    let mut call_edges = 0_usize;
    for (module_index, file) in input.syntax().files().iter().enumerate() {
        for (function_index, function) in file.functions().iter().enumerate() {
            let Some(caller) = offsets
                .get(module_index)
                .copied()
                .and_then(|offset| offset.checked_add(function_index))
            else {
                errors.global(
                    "ZRYNA-M3201",
                    "borrow-call graph indexing overflows checked arithmetic",
                    "reduce private functions with borrow-parameter calls",
                );
                return;
            };
            for expression in &function.body.expressions {
                let RawExpressionKind::Call { callee, .. } = &expression.kind else {
                    continue;
                };
                let FunctionResolution::Exact(signature) =
                    catalog.resolve(module_index, &callee.text)
                else {
                    continue;
                };
                if !signature.has_borrow_parameters() {
                    continue;
                }
                let at = span(input.sources(), expression.span);
                let Some(next_edges) = call_edges.checked_add(1) else {
                    errors.at(
                        "ZRYNA-M3201",
                        at,
                        "borrow-call edge counting overflows checked arithmetic",
                        "reduce private functions with borrow-parameter calls",
                    );
                    return;
                };
                call_edges = next_edges;
                if matches!(
                    borrow_call_program_budget_violation(call_edges, 1),
                    Some(BorrowCallProgramBudgetLimit::CallEdges)
                ) {
                    errors.at(
                        "ZRYNA-M3201",
                        at,
                        format!(
                            "borrow-call edges exceed the program limit of {}",
                            ir::MAX_CALL_EDGES
                        ),
                        "reduce private functions with borrow-parameter calls",
                    );
                    return;
                }
                let Some(target_module) = usize::try_from(signature.id.module.0).ok() else {
                    continue;
                };
                let Some(target_declaration) = usize::try_from(signature.id.declaration).ok()
                else {
                    continue;
                };
                let Some(target_offset) = offsets.get(target_module).copied() else {
                    continue;
                };
                let Some(target) = target_offset.checked_add(target_declaration) else {
                    errors.at(
                        "ZRYNA-M3201",
                        at,
                        "borrow-call graph indexing overflows checked arithmetic",
                        "reduce private functions with borrow-parameter calls",
                    );
                    return;
                };
                if target >= function_count {
                    errors.at(
                        "ZRYNA-M3201",
                        at,
                        "borrow-call graph target is outside checked indexing",
                        "reduce private functions with borrow-parameter calls",
                    );
                    return;
                }
                let Some(caller_edges) = graph.get_mut(caller) else {
                    continue;
                };
                caller_edges.push((target, at));
            }
        }
    }

    let mut indegree = vec![0_usize; function_count];
    for edges in &graph {
        for (target, _) in edges {
            let Some(target_indegree) = indegree.get_mut(*target) else {
                continue;
            };
            let Some(next) = target_indegree.checked_add(1) else {
                errors.global(
                    "ZRYNA-M3201",
                    "borrow-call graph indegree overflows checked arithmetic",
                    "reduce private functions with borrow-parameter calls",
                );
                return;
            };
            *target_indegree = next;
        }
    }
    let mut queue = indegree
        .iter()
        .enumerate()
        .filter_map(|(index, degree)| (*degree == 0).then_some(index))
        .collect::<VecDeque<_>>();
    let mut depth = vec![1_usize; function_count];
    while let Some(caller) = queue.pop_front() {
        for (target, at) in &graph[caller] {
            let Some(candidate) = depth[caller].checked_add(1) else {
                errors.at(
                    "ZRYNA-M3201",
                    *at,
                    "borrow-call depth overflows checked arithmetic",
                    "reduce the private borrow-call chain",
                );
                return;
            };
            depth[*target] = depth[*target].max(candidate);
            if matches!(
                borrow_call_program_budget_violation(call_edges, depth[*target]),
                Some(BorrowCallProgramBudgetLimit::CallDepth)
            ) {
                errors.at(
                    "ZRYNA-M3201",
                    *at,
                    format!(
                        "borrow-call depth exceeds the static limit of {}",
                        ir::MAX_STATIC_CALL_DEPTH
                    ),
                    "reduce the private borrow-call chain",
                );
                return;
            }
            indegree[*target] -= 1;
            if indegree[*target] == 0 {
                queue.push_back(*target);
            }
        }
    }
}
