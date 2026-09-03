use super::super::owned_string_lowering::PrivateStringLowerer;
use super::*;

#[test]
fn owned_place_preflight_is_source_located_and_string_temporary_failure_is_atomic() {
    let sources = sources_for(STRING_SOURCE);
    let syntax = verify_snapshot(response_snapshot(STRING_RESPONSE), &sources).expect("String v4");
    let input = pair_input(&syntax, &sources);
    let ty = authenticated_type_capabilities(input, 0, 0).expect("String type");
    let function = &syntax.files()[0].functions()[0];
    let catalog = FunctionCatalog { modules: vec![vec![]] };
    let at = span(&sources, zryna_source::UntrustedSpan { file: 0, start: 32, end: 35 });
    let maximum = zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION;

    let mut errors = Errors::new(&sources);
    assert!(preflight_owned_place_capacity(maximum - 1, 1, at, &mut errors));
    assert!(!preflight_owned_place_capacity_with_reserved(maximum - 1, 1, 1, at, &mut errors,));
    assert_eq!(errors.finish()[0].primary_span(), Some(at));
    let mut errors = Errors::new(&sources);
    let cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry");
    let mut lowerer = PrivateStringLowerer {
        input,
        function,
        module: 0,
        ty,
        catalog: &catalog,
        errors: &mut errors,
        bindings: std::collections::BTreeMap::new(),
        places: vec![
            raw::Place {
                id: raw::PlaceId(0),
                ty: ty.ir,
                span: at,
                kind: raw::PlaceKind::Local(0),
            };
            maximum - 1
        ],
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
    assert!(lowerer.reserve_local_place(at));
    assert!(
        lowerer
            .push_temporary(
                at,
                raw::InstructionKind::StringFromUtf8 {
                    bytes: b"x".to_vec(),
                    cleanup: raw::CleanupPlanId(0),
                },
            )
            .is_none()
    );
    assert_eq!(lowerer.places.len(), maximum - 1);
    assert_eq!(lowerer.reserved_places, 1);
    assert_eq!(lowerer.next_value, 0);
    assert_eq!(lowerer.cfg.transitions, 0);
    assert!(lowerer.cfg.current_block().expect("entry").instructions.is_empty());
    assert!(lowerer.owners.pending().is_empty());
    lowerer.release_local_place();
    drop(lowerer);
    let diagnostics = errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
    assert_eq!(diagnostics[0].primary_span(), Some(at));

    let mut overflow_errors = Errors::new(&sources);
    assert!(!preflight_owned_place_capacity(usize::MAX, 1, at, &mut overflow_errors));
    let diagnostics = overflow_errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
    assert_eq!(diagnostics[0].primary_span(), Some(at));
}

#[test]
#[allow(clippy::too_many_lines)]
fn private_string_nested_identity_result_reservation_fails_before_argument_mutation() {
    let (source, raw) = private_string_call_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful String calls");
    let input = pair_input(&syntax, &sources);
    let ty = authenticated_type_capabilities(input, 0, 0).expect("String type");
    let function = &syntax.files()[0].functions()[0];
    let signature = |declaration, name: &str, parameters: Vec<super::super::Ty>| {
        let parameter_order = (0..parameters.len())
            .map(|index| {
                FunctionParameterOrder::Value(u32::try_from(index).expect("parameter index"))
            })
            .collect();
        FunctionSignature {
            id: raw::FunctionId { module: raw::ModuleId(0), declaration },
            name: name.to_owned(),
            parameters,
            borrow_parameters: Vec::new(),
            parameter_order,
            result: ty,
            private: true,
        }
    };
    let catalog = FunctionCatalog {
        modules: vec![vec![
            Some(signature(0, "caller", Vec::new())),
            Some(signature(1, "identity", vec![ty])),
            Some(signature(2, "producer", Vec::new())),
        ]],
    };
    let expression = &function.body.expressions[2];
    let zryna_syntax::v4::RawExpressionKind::Call { callee, arguments, .. } = &expression.kind
    else {
        panic!("identity call")
    };
    let at = span(&sources, expression.span);
    let mut errors = Errors::new(&sources);
    let mut cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry");
    let maximum = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION;
    cfg.value_types.resize(maximum - 1, ty.ir);
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
        next_value: u32::try_from(maximum - 1).expect("value limit"),
        next_local: 0,
    };
    assert!(lowerer.direct_call(callee, arguments, at).is_none());
    assert_eq!(lowerer.cfg.value_types.len(), maximum - 1);
    assert_eq!(lowerer.cfg.transitions, 0);
    assert_eq!(lowerer.cfg.reserved_values, 0);
    assert_eq!(lowerer.cfg.reserved_transitions, 0);
    assert!(lowerer.cfg.current_block().expect("entry").instructions.is_empty());
    assert!(lowerer.places.is_empty());
    assert_eq!(lowerer.reserved_places, 0);
    assert!(lowerer.owners.pending().is_empty());
    assert!(lowerer.known_bytes.is_empty());
    assert!(lowerer.cleanup_plans.is_empty());
    drop(lowerer);
    let diagnostics = errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
    assert_eq!(diagnostics[0].primary_span(), Some(at));

    let zryna_syntax::v4::RawExpressionKind::Reference { name } =
        &function.body.expressions[3].kind
    else {
        panic!("call fixture reference")
    };
    let mut cleanup_errors = Errors::new(&sources);
    let mut cleanup = private_string_branch_budget_lowerer(
        input,
        function,
        ty,
        &catalog,
        &mut cleanup_errors,
        at,
        0,
    );
    cleanup.bindings.insert(
        name.text.clone(),
        super::super::Binding { ty, place: raw::PlaceId(0), mutable: false },
    );
    cleanup.places = vec![raw::Place {
        id: raw::PlaceId(0),
        ty: ty.ir,
        span: at,
        kind: raw::PlaceKind::Local(0),
    }];
    cleanup.owners = OwnerState {
        pending: vec![raw::PlaceId(0)],
        value_owners: std::collections::BTreeMap::new(),
    };
    cleanup.known_bytes.insert(raw::PlaceId(0), Some(1));
    cleanup.reserved_cleanup_plans = zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION;
    let before = (
        cleanup.bindings.clone(),
        cleanup.owners.clone(),
        cleanup.known_bytes.clone(),
        cleanup.cfg.current_block().expect("entry").instructions.clone(),
    );
    assert!(cleanup.direct_call(callee, &[3], at).is_none());
    assert_eq!(
        (
            cleanup.bindings.clone(),
            cleanup.owners.clone(),
            cleanup.known_bytes.clone(),
            cleanup.cfg.current_block().expect("entry").instructions.clone(),
        ),
        before
    );
    assert_eq!(cleanup.cfg.reserved_values, 0);
    assert_eq!(cleanup.cfg.reserved_transitions, 0);
    assert_eq!(cleanup.reserved_places, 0);
    drop(cleanup);
    let diagnostics = cleanup_errors.finish();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
    assert_eq!(diagnostics[0].primary_span(), Some(at));
}

#[test]
#[allow(clippy::too_many_lines)]
fn recursive_owned_string_preflight_is_exact_atomic_and_overflow_checked_for_all_consumers() {
    fn assert_boundaries(
        estimate: super::super::OwnedStringPreparationEstimate,
        ty: super::super::Ty,
        sources: &SourceMap,
        at: zryna_source::Span,
    ) {
        let values = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION - estimate.values;
        let transitions = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
            - estimate.transitions;
        let budget = OwnedStringPreparationBudget {
            cleanup_plans: zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION
                - estimate.cleanup_plans,
            reserved_cleanup_plans: 0,
            cleanup_actions: zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION
                - estimate.cleanup_actions,
            reserved_cleanup_actions: 0,
            places: zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION - estimate.places,
            reserved_places: 0,
        };
        let mut exact_errors = Errors::new(sources);
        let mut cfg = OwnedCfgState::single_block(at, &mut exact_errors).expect("entry");
        cfg.value_types.resize(values, ty.ir);
        cfg.transitions = transitions;
        let before =
            (cfg.value_types.len(), cfg.transitions, cfg.reserved_values, cfg.reserved_transitions);
        assert!(preflight_owned_string_preparation(
            estimate,
            budget,
            &mut cfg,
            at,
            &mut exact_errors,
        ));
        assert_eq!(
            (cfg.value_types.len(), cfg.transitions, cfg.reserved_values, cfg.reserved_transitions,),
            before
        );
        assert!(exact_errors.finish().is_empty());

        let mut extra_errors = Errors::new(sources);
        let mut extra_cfg = OwnedCfgState::single_block(at, &mut extra_errors).expect("entry");
        let extra_budget =
            OwnedStringPreparationBudget { cleanup_actions: budget.cleanup_actions + 1, ..budget };
        let before = (
            extra_cfg.value_types.len(),
            extra_cfg.transitions,
            extra_cfg.reserved_values,
            extra_cfg.reserved_transitions,
        );
        assert!(!preflight_owned_string_preparation(
            estimate,
            extra_budget,
            &mut extra_cfg,
            at,
            &mut extra_errors,
        ));
        assert_eq!(
            (
                extra_cfg.value_types.len(),
                extra_cfg.transitions,
                extra_cfg.reserved_values,
                extra_cfg.reserved_transitions,
            ),
            before
        );
        let diagnostics = extra_errors.finish();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
        assert_eq!(diagnostics[0].primary_span(), Some(at));
        assert_eq!(
            (diagnostics[0].message(), diagnostics[0].guidance()),
            (
                "recursive owned String preparation exceeds the per-function cleanup limits",
                "reduce nested String-producing expressions or simultaneously live owners",
            )
        );

        let mut overflow_errors = Errors::new(sources);
        let mut overflow_cfg =
            OwnedCfgState::single_block(at, &mut overflow_errors).expect("entry");
        let overflow_budget =
            OwnedStringPreparationBudget { cleanup_actions: usize::MAX, ..budget };
        assert!(!preflight_owned_string_preparation(
            estimate,
            overflow_budget,
            &mut overflow_cfg,
            at,
            &mut overflow_errors,
        ));
        let diagnostics = overflow_errors.finish();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
        assert_eq!(diagnostics[0].primary_span(), Some(at));
        assert_eq!(
            (diagnostics[0].message(), diagnostics[0].guidance()),
            (
                "recursive owned String preparation exceeds the per-function cleanup limits",
                "reduce nested String-producing expressions or simultaneously live owners",
            )
        );
    }

    let (call_source, call_raw) = private_nested_string_call_fixture();
    let call_sources = sources_for(&call_source);
    let call_syntax = verify_snapshot(call_raw, &call_sources).expect("nested call fixture");
    let call_input = pair_input(&call_syntax, &call_sources);
    let call_ty = authenticated_type_capabilities(call_input, 0, 0).expect("String type");
    let call_function = &call_syntax.files()[0].functions()[0];
    let mut bindings = std::collections::BTreeMap::new();
    bindings.insert(
        "survivor".to_owned(),
        super::super::Binding { ty: call_ty, place: raw::PlaceId(0), mutable: false },
    );
    let owners = OwnerState {
        pending: vec![raw::PlaceId(0)],
        value_owners: std::collections::BTreeMap::new(),
    };
    let call_estimate = estimate_owned_string_expression(
        call_function,
        &bindings,
        &owners,
        call_ty,
        4,
        1,
        OwnedStringEstimateContext::Value,
    )
    .expect("nested call estimate");
    assert_boundaries(
        call_estimate,
        call_ty,
        &call_sources,
        span(&call_sources, call_function.body.expressions[4].span),
    );

    let (vec_source, vec_raw) = private_vec_nested_string_fixture();
    let vec_sources = sources_for(&vec_source);
    let vec_syntax = verify_snapshot(vec_raw, &vec_sources).expect("nested Vec fixture");
    let vec_input = pair_input(&vec_syntax, &vec_sources);
    let vec_string = authenticated_type_capabilities(vec_input, 0, 0).expect("String type");
    let vec_function = &vec_syntax.files()[0].functions()[0];
    for (expression, pending) in [(2, 0), (7, 1)] {
        let owners = OwnerState {
            pending: (0..pending).map(raw::PlaceId).collect(),
            value_owners: std::collections::BTreeMap::new(),
        };
        let estimate = estimate_owned_string_expression(
            vec_function,
            &std::collections::BTreeMap::new(),
            &owners,
            vec_string,
            expression,
            usize::try_from(pending).expect("pending"),
            OwnedStringEstimateContext::Value,
        )
        .expect("nested Vec element estimate");
        assert_boundaries(
            estimate,
            vec_string,
            &vec_sources,
            span(
                &vec_sources,
                vec_function.body.expressions[usize::try_from(expression).expect("expression")]
                    .span,
            ),
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn nested_direct_call_uses_preflight_credit_without_conservative_double_counting() {
    let (source, raw_snapshot) = private_nested_string_call_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw_snapshot, &sources).expect("nested call fixture");
    let input = pair_input(&syntax, &sources);
    let ty = authenticated_type_capabilities(input, 0, 0).expect("String type");
    let function = &syntax.files()[0].functions()[0];
    let signature = |declaration, name: &str, parameters: Vec<super::super::Ty>| {
        let parameter_order = (0..parameters.len())
            .map(|index| {
                FunctionParameterOrder::Value(u32::try_from(index).expect("parameter index"))
            })
            .collect();
        FunctionSignature {
            id: raw::FunctionId { module: raw::ModuleId(0), declaration },
            name: name.to_owned(),
            parameters,
            borrow_parameters: Vec::new(),
            parameter_order,
            result: ty,
            private: true,
        }
    };
    let catalog = FunctionCatalog {
        modules: vec![vec![
            Some(signature(0, "caller", Vec::new())),
            Some(signature(1, "identity", vec![ty])),
            Some(signature(2, "producer", Vec::new())),
        ]],
    };
    let expression = &function.body.expressions[4];
    let zryna_syntax::v4::RawExpressionKind::Call { callee, arguments, .. } = &expression.kind
    else {
        panic!("identity call")
    };
    let at = span(&sources, expression.span);
    let mut bindings = std::collections::BTreeMap::new();
    bindings.insert(
        "survivor".to_owned(),
        super::super::Binding { ty, place: raw::PlaceId(0), mutable: false },
    );
    let owners = OwnerState {
        pending: vec![raw::PlaceId(0)],
        value_owners: std::collections::BTreeMap::new(),
    };
    let estimate = estimate_owned_string_expression(
        function,
        &bindings,
        &owners,
        ty,
        4,
        1,
        OwnedStringEstimateContext::Value,
    )
    .expect("nested call estimate");
    let value_base = zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION - estimate.values;
    let place_base = zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION - estimate.places;
    let mut errors = Errors::new(&sources);
    let mut cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry");
    cfg.value_types.resize(value_base, ty.ir);
    cfg.transitions =
        zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - estimate.transitions;
    let places = (0..place_base)
        .map(|index| raw::Place {
            id: raw::PlaceId(u32::try_from(index).expect("place id")),
            ty: ty.ir,
            span: at,
            kind: raw::PlaceKind::Local(u32::try_from(index).expect("local id")),
        })
        .collect();
    let mut lowerer = PrivateStringLowerer {
        input,
        function,
        module: 0,
        ty,
        catalog: &catalog,
        errors: &mut errors,
        bindings,
        places,
        reserved_places: 0,
        cfg,
        cleanup_plans: Vec::new(),
        cleanup_actions: 0,
        reserved_cleanup_plans: zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION
            - estimate.cleanup_plans,
        reserved_cleanup_actions: zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION
            - estimate.cleanup_actions,
        owners,
        known_bytes: std::collections::BTreeMap::from([(raw::PlaceId(0), Some(4))]),
        next_value: u32::try_from(value_base).expect("value id"),
        next_local: u32::try_from(place_base).expect("local id"),
    };
    let result = lowerer.direct_call(callee, arguments, at);
    assert!(result.is_some(), "exact nested preparation must not be double counted");
    assert_eq!(lowerer.cfg.value_types.len(), zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION);
    assert_eq!(lowerer.places.len(), zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION);
    assert_eq!(
        lowerer.cfg.transitions,
        zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
    );
    drop(lowerer);
    assert!(errors.finish().is_empty());
}
