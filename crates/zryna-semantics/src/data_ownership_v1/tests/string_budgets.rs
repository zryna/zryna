use super::super::owned_string_lowering::PrivateStringLowerer;
use super::*;

#[test]
fn static_string_concat_size_is_checked_at_exact_runtime_limit_and_overflow() {
    let max = zryna_ownership_runtime_abi::MAX_STRING_BYTES;
    assert_eq!(checked_string_concat_bytes(max - 1, 1), Some(max));
    assert_eq!(checked_string_concat_bytes(max, 1), None);
    assert_eq!(checked_string_concat_bytes(u64::MAX, 1), None);
}

#[test]
fn private_string_cleanup_action_budget_is_exact_and_checked_for_overflow() {
    let maximum = zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION;
    assert!(!cleanup_action_budget_violation(maximum - 1, 1, false));
    assert!(cleanup_action_budget_violation(maximum, 1, false));
    assert!(!cleanup_action_budget_violation(maximum, 1, true));
    assert!(cleanup_action_budget_violation(usize::MAX, 1, false));
}

#[test]
fn private_string_cleanup_action_overflow_is_source_located_m3201() {
    let sources = sources_for(STRING_SOURCE);
    let syntax = verify_snapshot(response_snapshot(STRING_RESPONSE), &sources).expect("String v4");
    let input = pair_input(&syntax, &sources);
    let ty = authenticated_type_capabilities(input, 0, 0).expect("String type");
    let function = &syntax.files()[0].functions()[0];
    let mut errors = Errors::new(&sources);
    let catalog = FunctionCatalog { modules: vec![vec![]] };
    let at = span(&sources, zryna_source::UntrustedSpan { file: 0, start: 32, end: 35 });
    let cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry block");
    let mut lowerer = PrivateStringLowerer {
        input,
        function,
        module: 0,
        ty,
        catalog: &catalog,
        errors: &mut errors,
        bindings: std::collections::BTreeMap::new(),
        places: Vec::new(),
        reserved_places: 0,
        cfg,
        cleanup_plans: Vec::new(),
        cleanup_actions: zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION,
        reserved_cleanup_plans: 0,
        reserved_cleanup_actions: 0,
        owners: OwnerState {
            pending: vec![raw::PlaceId(0)],
            value_owners: std::collections::BTreeMap::new(),
        },
        known_bytes: std::collections::BTreeMap::new(),
        next_value: 0,
        next_local: 0,
    };
    assert!(lowerer.push_cleanup(at, None).is_none());
    drop(lowerer);
    let diagnostics = errors.finish();
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
    let primary = diagnostics[0].primary_span().expect("cleanup site");
    assert_eq!((primary.start(), primary.end()), (32, 35));
    assert_eq!(
        diagnostics[0].message(),
        format!(
            "derived cleanup actions exceed the per-function M3 limit of {}",
            zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION
        )
    );
    assert_eq!(
        diagnostics[0].guidance(),
        "reduce simultaneously live Strings or fallible private String operations"
    );
}

#[test]
fn private_string_transition_limit_fails_before_external_lowerer_state_mutates() {
    let sources = sources_for(STRING_SOURCE);
    let syntax = verify_snapshot(response_snapshot(STRING_RESPONSE), &sources).expect("String v4");
    let input = pair_input(&syntax, &sources);
    let ty = authenticated_type_capabilities(input, 0, 0).expect("String type");
    let function = &syntax.files()[0].functions()[0];
    let mut errors = Errors::new(&sources);
    let catalog = FunctionCatalog { modules: vec![vec![]] };
    let at = span(&sources, zryna_source::UntrustedSpan { file: 0, start: 32, end: 35 });
    let mut cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry block");
    cfg.transitions = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION;
    let mut lowerer = PrivateStringLowerer {
        input,
        function,
        module: 0,
        ty,
        catalog: &catalog,
        errors: &mut errors,
        bindings: std::collections::BTreeMap::new(),
        places: Vec::new(),
        reserved_places: 0,
        cfg,
        cleanup_plans: Vec::new(),
        cleanup_actions: 0,
        reserved_cleanup_plans: 0,
        reserved_cleanup_actions: 0,
        owners: OwnerState::default(),
        known_bytes: std::collections::BTreeMap::new(),
        next_value: 0,
        next_local: 0,
    };
    assert!(lowerer.value(0).is_none());
    assert_eq!(lowerer.next_value, 0);
    assert!(lowerer.places.is_empty());
    assert!(lowerer.cleanup_plans.is_empty());
    assert!(lowerer.owners.pending().is_empty());
    drop(lowerer);
    let diagnostics = errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
    assert_eq!(diagnostics[0].primary_span(), Some(at));
}
