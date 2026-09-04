use super::*;
use zryna_ir::data_ownership_v1::raw;

#[derive(Clone, Copy)]
enum Failure {
    Plans(usize),
    Actions(usize),
}

fn exercise(plans: usize, actions: usize, failure: Option<Failure>) {
    for _ in 0..2 {
        let mut expected = None;
        let errors = with_fixture(Fixture::Array, |lowerer, result| {
            // Authenticated source/semantic input, synthetic initial resource counters.
            // These unused plans are a counter frontier, not a valid full-program IR proof.
            lowerer.cleanup_plans = (0..plans)
                .map(|id| raw::CleanupPlan {
                    id: raw::CleanupPlanId(u32::try_from(id).expect("bounded plan counter")),
                    span: span(lowerer.input.sources(), lowerer.function.span),
                    actions: Vec::new(),
                })
                .collect();
            lowerer.cleanup_actions = actions;
            let literals = lowerer
                .function
                .body
                .expressions
                .iter()
                .filter(|expression| {
                    matches!(expression.kind, RawExpressionKind::StringLiteral { .. })
                })
                .map(|expression| span(lowerer.input.sources(), expression.span))
                .collect::<Vec<_>>();
            assert_eq!(literals.len(), 2);
            if let Some(failure) = failure {
                let (at, message, help) = match failure {
                    Failure::Plans(index) => (
                        literals[index],
                        format!(
                            "derived cleanup sites exceed the per-function M3 limit of {}",
                            ir::MAX_CLEANUP_PLANS_PER_FUNCTION
                        ),
                        "reduce fallible String leaves in private aggregate construction",
                    ),
                    Failure::Actions(index) => (
                        literals[index],
                        format!(
                            "derived cleanup actions exceed the per-function M3 limit of {}",
                            ir::MAX_DROP_ACTIONS_PER_FUNCTION
                        ),
                        "reduce simultaneously live owned aggregates and String leaves",
                    ),
                };
                expected = Some(Diagnostic::error_at("ZRYNA-M3201", at, message, help));
            }
            let before = child_preparation_red::state(lowerer);
            let value = lowerer.value(root_value(lowerer, 0), result);
            if failure.is_some() {
                assert!(value.is_none());
                assert_eq!(
                    child_preparation_red::state(lowerer),
                    before,
                    "cleanup rejection preserves complete real state, including cache and credits"
                );
            } else {
                assert_eq!(value, Some(raw::ValueId(2)));
                assert_eq!(lowerer.cleanup_plans.len(), plans + 2);
                assert_eq!(lowerer.cleanup_actions, actions + 1);
                assert_eq!(lowerer.instructions.len(), 3);
                assert_eq!(lowerer.owners.pending(), &[raw::PlaceId(2)]);
                assert!(lowerer.constructor_storage_is_clear());
            }
        });
        assert_eq!(errors, expected.into_iter().collect::<Vec<_>>());
    }
}

#[test]
fn constructor_preparation_cleanup_plan_exact_extra_and_competing_limits_are_atomic() {
    let limit = ir::MAX_CLEANUP_PLANS_PER_FUNCTION;
    exercise(limit - 2, 0, None);
    exercise(limit - 1, 0, Some(Failure::Plans(1)));
    exercise(limit, usize::MAX, Some(Failure::Plans(0)));
    exercise(limit + 1, 0, Some(Failure::Plans(0)));
}

#[test]
fn constructor_preparation_cleanup_actions_exact_extra_and_overflow_are_atomic() {
    let limit = ir::MAX_DROP_ACTIONS_PER_FUNCTION;
    exercise(0, limit - 1, None);
    exercise(0, limit, Some(Failure::Actions(1)));
    exercise(0, usize::MAX, Some(Failure::Actions(0)));
}
