use super::*;

#[test]
#[allow(clippy::too_many_lines)]
fn nested_vec_construct_and_push_cleanup_fail_before_lowering_mutation_at_first_extra() {
    let (source, raw) = private_vec_nested_string_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful nested Vec elements");
    let input = pair_input(&syntax, &sources);
    let mut setup_errors = Errors::new(&sources);
    semantic_preflight(input, &mut setup_errors);
    let (graph, declarations) = super::super::build_graph(input, &mut setup_errors);
    let layouts = zryna_layout::verify(&graph, &sources, zryna_layout::StorageTarget::Linear32V1)
        .expect("verified layouts");
    let node_types = super::super::map_node_types(&graph, &layouts, &mut setup_errors);
    assert!(setup_errors.finish().is_empty());
    let vec_ty = authenticated_type_capabilities(input, 0, 1).expect("Vec<String>");
    let string_ty = authenticated_type_capabilities(input, 0, 0).expect("String");
    let file = &syntax.files()[0];
    let function = &file.functions()[0];
    let catalog = FunctionCatalog { modules: vec![vec![]] };

    for extra in [0, 1, 2] {
        let at = span(&sources, function.body.expressions[3].span);
        let mut errors = Errors::new(&sources);
        let cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry");
        let mut lowerer = super::super::PrivateVecLowerer {
            input,
            file,
            function,
            module: 0,
            declarations: &declarations,
            graph: &graph,
            node_types: &node_types,
            catalog: &catalog,
            vec_ty,
            element: string_ty,
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
            known_string_bytes: std::collections::BTreeMap::new(),
            next_value: 0,
            next_local: 0,
        };
        let estimate =
            lowerer.estimate_string_sequence(&[2], string_ty, at).expect("construct estimate");
        let outer_actions = estimate.end_pending;
        lowerer.reserved_cleanup_plans = zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION
            - estimate.cleanup_plans - 1
            + extra;
        lowerer.reserved_cleanup_actions = if extra == 2 {
            usize::MAX
        } else {
            zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION
                - estimate.cleanup_actions
                - outer_actions
                + extra
        };
        let before = (
            lowerer.bindings.clone(),
            lowerer.places.clone(),
            lowerer.cfg.current_block().expect("entry").instructions.clone(),
            lowerer.owners.clone(),
            lowerer.known_string_bytes.clone(),
            lowerer.cleanup_plans.clone(),
            lowerer.next_value,
            lowerer.next_local,
        );
        let result = lowerer.value(3, vec_ty);
        if extra == 0 {
            assert!(result.is_some(), "exact construct cleanup capacity");
        } else {
            assert!(result.is_none(), "extra or overflow construct cleanup must fail");
            assert_eq!(
                (
                    lowerer.bindings.clone(),
                    lowerer.places.clone(),
                    lowerer.cfg.current_block().expect("entry").instructions.clone(),
                    lowerer.owners.clone(),
                    lowerer.known_string_bytes.clone(),
                    lowerer.cleanup_plans.clone(),
                    lowerer.next_value,
                    lowerer.next_local,
                ),
                before
            );
        }
        drop(lowerer);
        let diagnostics = errors.finish();
        if extra == 0 {
            assert!(diagnostics.is_empty());
        } else {
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
            assert_eq!(diagnostics[0].primary_span(), Some(at));
        }
    }

    for extra in [0, 1, 2] {
        let at = span(&sources, function.body.expressions[8].span);
        let mut errors = Errors::new(&sources);
        let cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry");
        let place = raw::Place {
            id: raw::PlaceId(0),
            ty: vec_ty.ir,
            span: at,
            kind: raw::PlaceKind::Local(0),
        };
        let mut lowerer = super::super::PrivateVecLowerer {
            input,
            file,
            function,
            module: 0,
            declarations: &declarations,
            graph: &graph,
            node_types: &node_types,
            catalog: &catalog,
            vec_ty,
            element: string_ty,
            errors: &mut errors,
            bindings: std::collections::BTreeMap::from([(
                "values".to_owned(),
                super::super::Binding { ty: vec_ty, place: raw::PlaceId(0), mutable: true },
            )]),
            places: vec![place],
            reserved_places: 0,
            cfg,
            cleanup_plans: Vec::new(),
            cleanup_actions: 0,
            reserved_cleanup_plans: 0,
            reserved_cleanup_actions: 0,
            owners: OwnerState {
                pending: vec![raw::PlaceId(0)],
                value_owners: std::collections::BTreeMap::new(),
            },
            known_string_bytes: std::collections::BTreeMap::new(),
            next_value: 0,
            next_local: 1,
        };
        let estimate =
            lowerer.estimate_string_sequence(&[7], string_ty, at).expect("push estimate");
        let outer_actions = estimate.end_pending;
        lowerer.reserved_cleanup_plans = zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION
            - estimate.cleanup_plans - 1
            + extra;
        lowerer.reserved_cleanup_actions = if extra == 2 {
            usize::MAX
        } else {
            zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION
                - estimate.cleanup_actions
                - outer_actions
                + extra
        };
        let before = (
            lowerer.bindings.clone(),
            lowerer.places.clone(),
            lowerer.cfg.current_block().expect("entry").instructions.clone(),
            lowerer.owners.clone(),
            lowerer.known_string_bytes.clone(),
            lowerer.cleanup_plans.clone(),
            lowerer.next_value,
            lowerer.next_local,
        );
        let result = lowerer.lower_push_effect_with_policy(8, None, false);
        if extra == 0 {
            assert!(result.is_some(), "exact push cleanup capacity");
        } else {
            assert!(result.is_none(), "extra or overflow push cleanup must fail");
            assert_eq!(
                (
                    lowerer.bindings.clone(),
                    lowerer.places.clone(),
                    lowerer.cfg.current_block().expect("entry").instructions.clone(),
                    lowerer.owners.clone(),
                    lowerer.known_string_bytes.clone(),
                    lowerer.cleanup_plans.clone(),
                    lowerer.next_value,
                    lowerer.next_local,
                ),
                before
            );
        }
        drop(lowerer);
        let diagnostics = errors.finish();
        if extra == 0 {
            assert!(diagnostics.is_empty());
        } else {
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
            assert_eq!(diagnostics[0].primary_span(), Some(at));
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn nested_vec_direct_call_cleanup_is_exact_and_atomic_before_argument_lowering() {
    let (source, raw) = private_vec_nested_string_call_fixture();
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful nested Vec call");
    let input = pair_input(&syntax, &sources);
    let mut setup_errors = Errors::new(&sources);
    semantic_preflight(input, &mut setup_errors);
    let (graph, declarations) = super::super::build_graph(input, &mut setup_errors);
    let layouts = zryna_layout::verify(&graph, &sources, zryna_layout::StorageTarget::Linear32V1)
        .expect("verified layouts");
    let node_types = super::super::map_node_types(&graph, &layouts, &mut setup_errors);
    assert!(setup_errors.finish().is_empty());
    let vec_ty = authenticated_type_capabilities(input, 0, 1).expect("Vec<String>");
    let string_ty = authenticated_type_capabilities(input, 0, 0).expect("String");
    let file = &syntax.files()[0];
    let function = &file.functions()[0];
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
            result: vec_ty,
            private: true,
        }
    };
    let catalog = FunctionCatalog {
        modules: vec![vec![
            Some(signature(0, "caller", Vec::new())),
            Some(signature(1, "identity", vec![vec_ty])),
            Some(signature(2, "producer", Vec::new())),
        ]],
    };
    let expression = &function.body.expressions[5];
    let zryna_syntax::v4::RawExpressionKind::Call { callee, arguments, .. } = &expression.kind
    else {
        panic!("identity call")
    };
    let at = span(&sources, expression.span);

    for extra in [0, 1, 2] {
        let mut errors = Errors::new(&sources);
        let cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry");
        let place = raw::Place {
            id: raw::PlaceId(0),
            ty: vec_ty.ir,
            span: at,
            kind: raw::PlaceKind::Local(0),
        };
        let mut lowerer = super::super::PrivateVecLowerer {
            input,
            file,
            function,
            module: 0,
            declarations: &declarations,
            graph: &graph,
            node_types: &node_types,
            catalog: &catalog,
            vec_ty,
            element: string_ty,
            errors: &mut errors,
            bindings: std::collections::BTreeMap::from([(
                "survivor".to_owned(),
                super::super::Binding { ty: vec_ty, place: raw::PlaceId(0), mutable: false },
            )]),
            places: vec![place],
            reserved_places: 0,
            cfg,
            cleanup_plans: Vec::new(),
            cleanup_actions: 0,
            reserved_cleanup_plans: 0,
            reserved_cleanup_actions: 0,
            owners: OwnerState {
                pending: vec![raw::PlaceId(0)],
                value_owners: std::collections::BTreeMap::new(),
            },
            known_string_bytes: std::collections::BTreeMap::new(),
            next_value: 0,
            next_local: 1,
        };
        let preparation = lowerer
            .estimate_vec_preparation(4, vec_ty, 1, at)
            .expect("nested Vec argument estimate");
        let outer_actions = preparation.end_pending - 1;
        lowerer.reserved_cleanup_plans = zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION
            - preparation.resources.cleanup_plans - 1
            + extra;
        lowerer.reserved_cleanup_actions = if extra == 2 {
            usize::MAX
        } else {
            zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION
                - preparation.resources.cleanup_actions
                - outer_actions
                + extra
        };
        let before = (
            lowerer.bindings.clone(),
            lowerer.places.clone(),
            lowerer.cfg.current_block().expect("entry").instructions.clone(),
            lowerer.owners.clone(),
            lowerer.known_string_bytes.clone(),
            lowerer.cleanup_plans.clone(),
            lowerer.next_value,
            lowerer.next_local,
        );
        let result = lowerer.direct_call(callee, arguments, vec_ty, at);
        if extra == 0 {
            assert!(result.is_some(), "exact nested Vec call cleanup capacity");
        } else {
            assert!(result.is_none(), "extra or overflow nested Vec call must fail");
            assert_eq!(
                (
                    lowerer.bindings.clone(),
                    lowerer.places.clone(),
                    lowerer.cfg.current_block().expect("entry").instructions.clone(),
                    lowerer.owners.clone(),
                    lowerer.known_string_bytes.clone(),
                    lowerer.cleanup_plans.clone(),
                    lowerer.next_value,
                    lowerer.next_local,
                ),
                before
            );
        }
        drop(lowerer);
        let diagnostics = errors.finish();
        if extra == 0 {
            assert!(diagnostics.is_empty());
        } else {
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
            assert_eq!(diagnostics[0].primary_span(), Some(at));
        }
    }
}
