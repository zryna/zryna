use super::*;

#[test]
fn vec_operand_amplification_budget_is_exact_and_plus_one_fails() {
    assert!(!aggregate_operand_budget_violation(
        zryna_ir::data_ownership_v1::MAX_AGGREGATE_OPERANDS - 1,
        1,
    ));
    assert!(aggregate_operand_budget_violation(
        zryna_ir::data_ownership_v1::MAX_AGGREGATE_OPERANDS,
        1,
    ));
    assert!(aggregate_operand_budget_violation(usize::MAX, 1));
}

#[test]
fn vec_derived_values_and_resource_additions_have_exact_checked_boundaries() {
    let raw = response_snapshot(VEC_STRING_RESPONSE);
    assert_eq!(derived_value_count(&raw.files[0].functions[0]), 5);
    for maximum in [
        zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION,
        zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION,
        zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION,
        zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION,
    ] {
        assert!(!resource_budget_violation(maximum - 1, 1, maximum));
        assert!(resource_budget_violation(maximum, 1, maximum));
        assert!(resource_budget_violation(usize::MAX, 1, maximum));
    }
}

#[test]
fn owned_aggregate_operand_budget_is_exact_plus_one_and_overflow_checked() {
    let maximum = zryna_ir::data_ownership_v1::MAX_AGGREGATE_OPERANDS;
    assert!(!aggregate_operand_budget_violation(maximum, 0));
    assert!(!aggregate_operand_budget_violation(maximum - 1, 1));
    assert!(aggregate_operand_budget_violation(maximum, 1));
    assert!(aggregate_operand_budget_violation(usize::MAX, 1));

    let sources = sources_for(OWNED_PAIR_SOURCE);
    let at = sources
        .verify_span(zryna_source::UntrustedSpan { file: 0, start: 121, end: 158 })
        .expect("constructor span");
    let mut exact_errors = Errors::new(&sources);
    assert_eq!(
        preflight_aggregate_operand_total(maximum - 2, 2, at, &mut exact_errors),
        Some(maximum),
    );
    assert!(exact_errors.is_empty());
    let mut extra_errors = Errors::new(&sources);
    assert_eq!(preflight_aggregate_operand_total(maximum, 1, at, &mut extra_errors), None);
    let diagnostics = extra_errors.finish();
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
    assert_eq!(
        diagnostics[0].primary_span().map(|span| (span.start(), span.end())),
        Some((121, 158)),
    );
}
