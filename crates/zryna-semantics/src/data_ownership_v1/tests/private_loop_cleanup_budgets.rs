use super::super::owned_string_lowering::PrivateStringLowerer;
use super::*;

#[test]
fn private_string_loop_rejects_incoming_owner_move_at_reference_before_lowering() {
    let (source, raw) = private_string_loop_fixture_with_incoming_move(true);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful incoming loop move");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("incoming move must reject");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3015");
    let primary = diagnostics[0].primary_span().expect("incoming reference span");
    let expected = nth_untrusted_span(&source, "outer", 1);
    assert_eq!((primary.start(), primary.end()), (expected.start, expected.end));
}
#[test]
fn private_string_loop_rejects_non_bool_condition_at_exact_reference() {
    let (source, raw) = private_string_loop_fixture_with_options(false, true, false);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful non-bool loop condition");
    let diagnostics = lower(pair_input(&syntax, &sources)).expect_err("non-bool loop must reject");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3012");
    let primary = diagnostics[0].primary_span().expect("condition reference span");
    let expected = nth_untrusted_span(&source, "outer", 1);
    assert_eq!((primary.start(), primary.end()), (expected.start, expected.end));
}
#[test]
fn private_string_false_loop_retains_reachable_exit_and_replays_deterministically() {
    let (source, raw) = private_string_loop_fixture_with_options(false, false, true);
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful false loop");
    let first = lower(pair_input(&syntax, &sources)).expect("false loop must retain its exit");
    let second = lower(pair_input(&syntax, &sources)).expect("false loop replay must verify");
    assert_eq!(format!("{:?}", first.verified_ir()), format!("{:?}", second.verified_ir()));
    let function =
        first.verified_ir().modules().next().expect("module").functions().next().expect("function");
    let blocks = function.blocks().collect::<Vec<_>>();
    assert_eq!(blocks.len(), 4);
    assert_eq!(blocks[1].instructions().next().expect("header false").bool_literal(), Some(false));
    assert_eq!(blocks[3].terminator().kind(), VerifiedTerminatorKind::Return);
}
#[test]
fn owned_loop_shape_preflight_rejects_nested_return_repetition_and_post_effect() {
    let (source, mut raw) = private_string_loop_fixture();
    let sources = sources_for(&source);
    let function = &mut raw.files[0].functions[0];
    let body_statement_span = function.body.statements[2].span;
    let RawStatementKind::LocalDeclaration { initializer, semicolon_span, .. } =
        function.body.statements[2].kind
    else {
        unreachable!("fixture body local")
    };
    function.body.statements[2].kind = RawStatementKind::Return {
        keyword_span: body_statement_span,
        value: initializer,
        semicolon_span,
    };
    let mut errors = Errors::new(&sources);
    assert!(!preflight_owned_loop_body(function, 1, false, &sources, &mut errors));
    let diagnostics = errors.finish();
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
    assert_eq!(diagnostics[0].primary_span(), Some(span(&sources, body_statement_span)));

    let (source, mut raw) = private_string_loop_fixture();
    let sources = sources_for(&source);
    let function = &mut raw.files[0].functions[0];
    let body_statement_span = function.body.statements[2].span;
    function.body.statements[2].kind = RawStatementKind::While {
        keyword_span: body_statement_span,
        open_paren_span: body_statement_span,
        condition: 1,
        close_paren_span: body_statement_span,
        body_block: 1,
    };
    let mut errors = Errors::new(&sources);
    assert!(!preflight_owned_loop_body(function, 1, false, &sources, &mut errors));
    let diagnostics = errors.finish();
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
    assert_eq!(diagnostics[0].primary_span(), Some(span(&sources, body_statement_span)));

    let (source, mut raw) = private_string_loop_fixture();
    let sources = sources_for(&source);
    let function = &mut raw.files[0].functions[0];
    function.body.blocks[0].statements = vec![0, 1, 1, 4];
    let repeated_span = function.body.statements[1].span;
    let mut errors = Errors::new(&sources);
    assert!(!preflight_owned_loop_exit(function, 1, &sources, &mut errors));
    let diagnostics = errors.finish();
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
    assert_eq!(diagnostics[0].primary_span(), Some(span(&sources, repeated_span)));

    let (source, mut raw) = private_string_loop_fixture();
    let sources = sources_for(&source);
    let function = &mut raw.files[0].functions[0];
    function.body.blocks[0].statements = vec![0, 1, 2, 4];
    let effect_span = function.body.statements[2].span;
    let mut errors = Errors::new(&sources);
    assert!(!preflight_owned_loop_exit(function, 1, &sources, &mut errors));
    let diagnostics = errors.finish();
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3016");
    assert_eq!(diagnostics[0].primary_span(), Some(span(&sources, effect_span)));
}
#[test]
fn vec_cleanup_reservations_are_expression_aware_at_exact_boundaries() {
    let maximum = zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION;
    assert_eq!(cleanup_actions_after_preparation(maximum, false), maximum);
    assert!(!resource_budget_violation(
        0,
        cleanup_actions_after_preparation(maximum, false),
        maximum
    ));
    assert!(resource_budget_violation(
        0,
        cleanup_actions_after_preparation(maximum, true),
        maximum
    ));
    assert_eq!(cleanup_actions_after_transfer(maximum, true), maximum - 1);
    assert!(!resource_budget_violation(1, cleanup_actions_after_transfer(maximum, true), maximum));
    assert!(resource_budget_violation(1, cleanup_actions_after_transfer(maximum, false), maximum));
    assert_eq!(cleanup_actions_after_preparation(usize::MAX, true), usize::MAX);
    assert_eq!(cleanup_actions_after_transfer(0, true), 0);
    assert_eq!(cleanup_actions_after_additions(maximum, 0), maximum);
    assert!(resource_budget_violation(0, cleanup_actions_after_additions(maximum, 1), maximum));
}
pub(super) fn private_string_branch_budget_lowerer<'a, 'e>(
    input: SemanticInput<'a>,
    function: &'a RawFunctionSyntax,
    ty: super::super::Ty,
    catalog: &'a FunctionCatalog,
    errors: &'e mut Errors<'a>,
    at: zryna_source::Span,
    cleanup_actions: usize,
) -> PrivateStringLowerer<'a, 'a, 'e> {
    let owners = OwnerState {
        pending: vec![raw::PlaceId(0), raw::PlaceId(1), raw::PlaceId(2)],
        ..OwnerState::default()
    };
    let cfg = OwnedCfgState::single_block(at, errors).expect("entry block");
    PrivateStringLowerer {
        input,
        function,
        module: 0,
        ty,
        catalog,
        errors,
        bindings: std::collections::BTreeMap::new(),
        places: Vec::new(),
        reserved_places: 0,
        cfg,
        cleanup_plans: Vec::new(),
        cleanup_actions,
        reserved_cleanup_plans: 0,
        reserved_cleanup_actions: 0,
        owners,
        known_bytes: std::collections::BTreeMap::new(),
        next_value: 0,
        next_local: 0,
    }
}

fn assert_m3201_at(diagnostic: &zryna_diagnostics::Diagnostic, at: zryna_source::Span) {
    assert_eq!(diagnostic.code(), "ZRYNA-M3201");
    assert_eq!(diagnostic.primary_span(), Some(at));
}

fn cleanup_action_limit_message() -> String {
    format!(
        "derived cleanup actions exceed the per-function M3 limit of {}",
        zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION
    )
}

fn assert_cleanup_diagnostic(
    diagnostic: &zryna_diagnostics::Diagnostic,
    at: zryna_source::Span,
    message: &str,
    guidance: &str,
) {
    assert_m3201_at(diagnostic, at);
    assert_eq!(diagnostic.message(), message);
    assert_eq!(diagnostic.guidance(), guidance);
}

fn assert_cleanup_action_context_diagnostics(sources: &SourceMap, at: zryna_source::Span) {
    let action_contexts = [
        (
            OwnedCleanupActionContext::StringBranchLocal,
            cleanup_action_limit_message(),
            "reduce branch-local owned Strings or fallible String operations",
        ),
        (
            OwnedCleanupActionContext::StringTerminalArm,
            "terminal String arm cleanup exceeds the per-function M3 limit".to_owned(),
            "reduce owned temporaries in the returning branch expression",
        ),
        (
            OwnedCleanupActionContext::VecBranchLocal,
            cleanup_action_limit_message(),
            "reduce branch-local owned values or fallible Vec operations",
        ),
        (
            OwnedCleanupActionContext::VecTerminalArm,
            "terminal Vec arm cleanup exceeds the per-function M3 limit".to_owned(),
            "reduce owned temporaries in the returning branch expression",
        ),
    ];
    for (context, message, guidance) in action_contexts {
        let mut errors = Errors::new(sources);
        let mut plans = Vec::new();
        let mut committed = zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION;
        let mut reserved_plans = 0;
        let mut reserved_actions = 0;
        let accounting = OwnedCleanupAccounting::new(
            &mut plans,
            &mut committed,
            &mut reserved_plans,
            &mut reserved_actions,
        );
        assert!(!accounting.preflight_actions(1, context, at, &mut errors));
        assert!(plans.is_empty());
        assert_eq!(committed, zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION);
        assert_eq!((reserved_plans, reserved_actions), (0, 0));
        let diagnostics = errors.finish();
        assert_eq!(diagnostics.len(), 1);
        assert_cleanup_diagnostic(&diagnostics[0], at, &message, guidance);
    }
}

#[test]
fn private_string_branch_drop_budget_is_atomic_at_exact_plus_one() {
    let sources = sources_for(STRING_SOURCE);
    let syntax = verify_snapshot(response_snapshot(STRING_RESPONSE), &sources).expect("String v4");
    let input = pair_input(&syntax, &sources);
    let ty = authenticated_type_capabilities(input, 0, 0).expect("String type");
    let function = &syntax.files()[0].functions()[0];
    let catalog = FunctionCatalog { modules: vec![vec![]] };
    let at = span(&sources, zryna_source::UntrustedSpan { file: 0, start: 32, end: 35 });
    let incoming = OwnedStringBranchState {
        bindings: std::collections::BTreeMap::new(),
        owners: OwnerState {
            pending: vec![raw::PlaceId(0)],
            value_owners: std::collections::BTreeMap::new(),
        },
        known_bytes: std::collections::BTreeMap::new(),
    };

    let mut exact_errors = Errors::new(&sources);
    let mut exact = private_string_branch_budget_lowerer(
        input,
        function,
        ty,
        &catalog,
        &mut exact_errors,
        at,
        zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION - 2,
    );
    exact.cfg.transitions = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 2;
    assert!(exact.restore_branch_scope(&incoming, at).is_some());
    assert_eq!(exact.cleanup_actions, zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION);
    assert_eq!(exact.owners, incoming.owners);
    drop(exact);
    assert!(exact_errors.finish().is_empty());

    let mut extra_errors = Errors::new(&sources);
    let mut extra = private_string_branch_budget_lowerer(
        input,
        function,
        ty,
        &catalog,
        &mut extra_errors,
        at,
        zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION - 1,
    );
    let before = extra.owners.clone();
    assert!(extra.restore_branch_scope(&incoming, at).is_none());
    assert_eq!(
        extra.cleanup_actions,
        zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION - 1
    );
    assert_eq!(extra.owners, before);
    assert!(extra.cfg.current_block().expect("entry").instructions.is_empty());
    drop(extra);
    let diagnostics = extra_errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_cleanup_diagnostic(
        &diagnostics[0],
        at,
        &cleanup_action_limit_message(),
        "reduce branch-local owned Strings or fallible String operations",
    );

    let mut transition_errors = Errors::new(&sources);
    let mut transition = private_string_branch_budget_lowerer(
        input,
        function,
        ty,
        &catalog,
        &mut transition_errors,
        at,
        0,
    );
    transition.cfg.transitions =
        zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - 1;
    let before = transition.owners.clone();
    assert!(transition.restore_branch_scope(&incoming, at).is_none());
    assert_eq!(transition.cleanup_actions, 0);
    assert_eq!(transition.owners, before);
    assert!(transition.cfg.current_block().expect("entry").instructions.is_empty());
    drop(transition);
    let diagnostics = transition_errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_m3201_at(&diagnostics[0], at);

    let mut overflow_errors = Errors::new(&sources);
    let mut overflow = private_string_branch_budget_lowerer(
        input,
        function,
        ty,
        &catalog,
        &mut overflow_errors,
        at,
        0,
    );
    overflow.cfg.transitions = usize::MAX;
    let before = overflow.owners.clone();
    assert!(overflow.restore_branch_scope(&incoming, at).is_none());
    assert_eq!(overflow.cleanup_actions, 0);
    assert_eq!(overflow.owners, before);
    assert!(overflow.cfg.current_block().expect("entry").instructions.is_empty());
    drop(overflow);
    let diagnostics = overflow_errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_m3201_at(&diagnostics[0], at);
    assert_cleanup_action_context_diagnostics(&sources, at);
}
