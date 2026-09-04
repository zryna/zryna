use super::*;
use crate::data_ownership_v1::owned_aggregate_lowering::resource_decisions::AggregateUsage;
use zryna_layout::TypeCategory;

#[path = "aggregate_credit_bridges.rs"]
mod credits;

fn usage(dimension: usize, raw: usize, held: usize) -> AggregateUsage {
    let mut result = AggregateUsage {
        values: 0,
        places: 0,
        transitions: 0,
        operands: 0,
        held_values: 0,
        held_places: 0,
        held_transitions: 0,
        held_operands: 0,
    };
    match dimension {
        0 => {
            result.operands = raw;
            result.held_operands = held;
        }
        1 => {
            result.transitions = raw;
            result.held_transitions = held;
        }
        2 => {
            result.values = raw;
            result.held_values = held;
        }
        3 => {
            result.places = raw;
            result.held_places = held;
        }
        _ => unreachable!(),
    }
    result
}

fn counters(usage: &AggregateUsage) -> [usize; 8] {
    [
        usage.operands,
        usage.transitions,
        usage.values,
        usage.places,
        usage.held_operands,
        usage.held_transitions,
        usage.held_values,
        usage.held_places,
    ]
}

fn select(
    usage: &AggregateUsage,
    dimension: usize,
    extra: usize,
    at: Span,
    errors: &mut Errors<'_>,
) -> bool {
    match dimension {
        0 => usage.operands(extra, at, errors),
        1 => usage.transition(extra, at, errors),
        2 => usage.value(at, errors),
        3 => usage.places(extra, at, errors),
        _ => unreachable!(),
    }
}

fn expected(dimension: usize, at: Span) -> Diagnostic {
    let (message, help) = match dimension {
        0 => (
            format!("derived aggregate operands exceed the M3 limit of {}", LIMITS[0]),
            "reduce Struct fields and fixed-array elements",
        ),
        1 => (
            format!(
                "derived ownership transitions exceed the per-function M3 limit of {}",
                LIMITS[1]
            ),
            "reduce private aggregate expressions and assignments",
        ),
        2 => (
            format!("derived values exceed the per-function M3 limit of {}", LIMITS[2]),
            "reduce private aggregate expressions",
        ),
        3 => (
            format!("derived places exceed the per-function M3 limit of {}", LIMITS[3]),
            "reduce owned aggregate temporaries and locals",
        ),
        _ => unreachable!(),
    };
    Diagnostic::error_at("ZRYNA-M3201", at, message, help)
}

fn state(lowerer: &PrivateOwnedAggregateLowerer<'_, '_, '_>) -> String {
    format!(
        "{:?}",
        (
            (&lowerer.instructions, &lowerer.places, &lowerer.cleanup_plans, &lowerer.owners),
            (
                &lowerer.bindings,
                &lowerer.projections,
                &lowerer.moved_projections,
                &lowerer.partial_roots
            ),
            (
                lowerer.next_value,
                lowerer.next_local,
                lowerer.cleanup_actions,
                lowerer.aggregate_operands,
                lowerer.aggregate_subobject_moves,
                lowerer.projected_aggregate_clones,
                lowerer.projected_aggregate_assignments,
                credits(lowerer)
            ),
        )
    )
}

#[test]
fn aggregate_resource_decisions_exact_extra_and_split_usage_are_read_only() {
    // Synthetic counter frontiers, not claims of complete valid raw IR at these counts.
    for (dimension, limit) in LIMITS.into_iter().enumerate() {
        let extra = if dimension == 0 { 2 } else { 1 };
        for split in [0, 1, 2] {
            for excess in [0, 1] {
                let mut diagnostic = None;
                let errors = with_fixture(Fixture::Pair, |lowerer, _| {
                    let at = span(lowerer.input.sources(), lowerer.function.span);
                    diagnostic = Some(expected(dimension, at));
                    let used = limit - extra + excess;
                    let (raw, held) = match split {
                        0 => (used, 0),
                        1 => (1, used - 1),
                        _ => (0, used),
                    };
                    let view = usage(dimension, raw, held);
                    let before = counters(&view);
                    let live = state(lowerer);
                    assert_eq!(select(&view, dimension, extra, at, lowerer.errors), excess == 0);
                    assert_eq!(counters(&view), before);
                    assert_eq!(state(lowerer), live);
                });
                if excess == 0 {
                    assert!(errors.is_empty());
                } else {
                    assert_eq!(errors, vec![diagnostic.expect("exact source span")]);
                }
            }
        }
    }
}

#[test]
fn aggregate_resource_decisions_raw_held_and_additional_overflow_are_rejected() {
    // A zero-sized request still validates existing committed-plus-held usage.
    for dimension in [0, 1, 3] {
        for held_only in [false, true] {
            for excess in [0, 1] {
                let mut diagnostic = None;
                let errors = with_fixture(Fixture::Pair, |lowerer, _| {
                    let at = span(lowerer.input.sources(), lowerer.function.span);
                    diagnostic = Some(expected(dimension, at));
                    let used = LIMITS[dimension] + excess;
                    let (raw, held) = if held_only { (0, used) } else { (used, 0) };
                    let view = usage(dimension, raw, held);
                    let before = counters(&view);
                    let live = state(lowerer);
                    assert_eq!(select(&view, dimension, 0, at, lowerer.errors), excess == 0);
                    assert_eq!(counters(&view), before);
                    assert_eq!(state(lowerer), live);
                });
                if excess == 0 {
                    assert!(errors.is_empty());
                } else {
                    assert_eq!(errors, vec![diagnostic.expect("exact source span")]);
                }
            }
        }
    }
    for dimension in 0..4 {
        for (raw, held, extra) in
            [(usize::MAX, 1, 1), (1, usize::MAX, 1), (0, usize::MAX, 1), (1, 0, usize::MAX)]
        {
            // value() always requests one value; its own raw/held overflow cases are above.
            if dimension == 2 && extra == usize::MAX {
                continue;
            }
            let mut diagnostic = None;
            let errors = with_fixture(Fixture::Pair, |lowerer, _| {
                let at = span(lowerer.input.sources(), lowerer.function.span);
                diagnostic = Some(expected(dimension, at));
                let view = usage(dimension, raw, held);
                let before = counters(&view);
                let live = state(lowerer);
                assert!(!select(&view, dimension, extra, at, lowerer.errors));
                assert_eq!(counters(&view), before);
                assert_eq!(state(lowerer), live);
            });
            assert_eq!(errors, vec![diagnostic.expect("exact source span")]);
        }
    }
}

#[test]
fn aggregate_resource_decisions_adapter_keeps_committed_and_held_counts_separate() {
    let errors = with_fixture(Fixture::Pair, |lowerer, result| {
        assert!(run_statement(lowerer, 0, result));
        set_credits(lowerer, [7, 11, 13, 17]);
        let before = state(lowerer);
        assert_eq!(
            counters(&lowerer.resource_usage()),
            [
                lowerer.aggregate_operands,
                lowerer.instructions.len(),
                lowerer.next_value as usize,
                lowerer.places.len(),
                7,
                11,
                13,
                17,
            ]
        );
        assert_eq!(state(lowerer), before);
    });
    assert!(errors.is_empty());
}

#[test]
fn aggregate_resource_decisions_live_wrappers_reject_without_effects() {
    for dimension in 0..4 {
        for held in [LIMITS[dimension], usize::MAX] {
            let mut diagnostic = None;
            let errors = with_fixture(Fixture::Pair, |lowerer, result| {
                assert!(run_statement(lowerer, 0, result));
                let mut credits = [0; 4];
                credits[dimension] = held;
                set_credits(lowerer, credits);
                let at = span(lowerer.input.sources(), lowerer.function.span);
                diagnostic = Some(expected(dimension, at));
                let before = state(lowerer);
                let accepted = match dimension {
                    0 => lowerer.preflight_constructor_operands(1, at),
                    1 => lowerer.preflight_transition(1, at),
                    2 => lowerer.preflight_value(at),
                    3 => lowerer.preflight_constructor_places(1, at),
                    _ => unreachable!(),
                };
                assert!(!accepted);
                assert_eq!(state(lowerer), before);
            });
            assert_eq!(errors, vec![diagnostic.expect("exact source span")]);
        }
    }
}

#[test]
fn aggregate_resource_decisions_constructor_failure_order_preserves_live_state() {
    for dimension in 0..4 {
        let mut diagnostic = None;
        let errors = with_fixture(Fixture::Pair, |lowerer, result| {
            assert!(run_statement(lowerer, 0, result));
            let mut held = LIMITS;
            held[..dimension].fill(0);
            set_credits(lowerer, held);
            let at = span(lowerer.input.sources(), lowerer.function.span);
            diagnostic = Some(expected(dimension, at));
            let before = state(lowerer);
            assert!(lowerer.reserve_constructor_commit(result, 2, at).is_none());
            assert_eq!(state(lowerer), before);
        });
        assert_eq!(errors, vec![diagnostic.expect("exact source span")]);
    }
}

#[test]
fn aggregate_resource_decisions_emit_failure_order_and_copy_place_skip_are_exact() {
    for dimension in 1..4 {
        let mut diagnostic = None;
        let errors = with_fixture(Fixture::Pair, |lowerer, result| {
            assert!(run_statement(lowerer, 0, result));
            let kind = lowerer.instructions.last().expect("constructor").kind.clone();
            let mut held = LIMITS;
            held[..dimension].fill(0);
            set_credits(lowerer, held);
            let at = span(lowerer.input.sources(), lowerer.function.span);
            diagnostic = Some(expected(dimension, at));
            let before = state(lowerer);
            assert!(lowerer.emit(result, at, kind).is_none());
            assert_eq!(state(lowerer), before);
        });
        assert_eq!(errors, vec![diagnostic.expect("exact source span")]);
    }
    let errors = with_fixture(Fixture::Pair, |lowerer, result| {
        assert!(run_statement(lowerer, 0, result));
        let copy = lowerer
            .node_types
            .iter()
            .flatten()
            .find(|ty| ty.category == TypeCategory::Bool)
            .copied()
            .expect("bool");
        let kind = lowerer
            .instructions
            .iter()
            .find(|instruction| {
                instruction.result.as_ref().is_some_and(|value| value.ty == copy.ir)
            })
            .expect("authentic Copy instruction")
            .kind
            .clone();
        set_credits(lowerer, [0, 0, 0, usize::MAX]);
        let places = lowerer.places.clone();
        let owners = lowerer.owners.clone();
        let next = lowerer.next_value;
        let instructions = lowerer.instructions.len();
        let at = span(lowerer.input.sources(), lowerer.function.span);
        let value = lowerer.emit(copy, at, kind).expect("Copy emission skips place capacity");
        assert_eq!(value.0, next);
        assert_eq!(lowerer.next_value, next + 1);
        assert_eq!(lowerer.instructions.len(), instructions + 1);
        assert_eq!(lowerer.places, places);
        assert_eq!(lowerer.owners, owners);
        assert_eq!(credits(lowerer), [0, 0, 0, usize::MAX]);
    });
    assert!(errors.is_empty());
}

#[test]
fn aggregate_resource_decisions_zero_place_constructor_check_is_not_emit_skip() {
    for excess in [0, 1] {
        let mut diagnostic = None;
        let errors = with_fixture(Fixture::Pair, |lowerer, _| {
            let copy = lowerer
                .node_types
                .iter()
                .flatten()
                .find(|ty| ty.category == TypeCategory::Bool)
                .copied()
                .expect("bool");
            set_credits(lowerer, [0, 0, 0, LIMITS[3] + excess]);
            let at = span(lowerer.input.sources(), lowerer.function.span);
            diagnostic = Some(expected(3, at));
            let before = state(lowerer);
            let ticket = lowerer.reserve_constructor_commit(copy, 0, at);
            assert_eq!(ticket.is_some(), excess == 0);
            if let Some(ticket) = ticket {
                ticket.release(lowerer);
            }
            assert_eq!(state(lowerer), before);
        });
        if excess == 0 {
            assert!(errors.is_empty());
        } else {
            assert_eq!(errors, vec![diagnostic.expect("exact source span")]);
        }
    }
}
