use super::*;

#[test]
#[allow(clippy::too_many_lines)]
fn private_vec_clone_preflights_exact_first_extra_and_overflow_atomically() {
    let (source, raw) = private_vec_clone_fixture("i32");
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful Vec clone");
    let input = pair_input(&syntax, &sources);
    let mut setup_errors = Errors::new(&sources);
    semantic_preflight(input, &mut setup_errors);
    let (graph, declarations) = super::super::build_graph(input, &mut setup_errors);
    let layouts = zryna_layout::verify(&graph, &sources, zryna_layout::StorageTarget::Linear32V1)
        .expect("verified layouts");
    let node_types = super::super::map_node_types(&graph, &layouts, &mut setup_errors);
    assert!(setup_errors.finish().is_empty());
    let vec_ty = node_types
        .iter()
        .flatten()
        .find(|ty| ty.category == zryna_layout::TypeCategory::Vec)
        .copied()
        .expect("Vec<i32>");
    let element = node_types
        .iter()
        .flatten()
        .find(|ty| ty.category == zryna_layout::TypeCategory::I32)
        .copied()
        .expect("i32");
    let file = &syntax.files()[0];
    let function = &file.functions()[0];
    let catalog = FunctionCatalog { modules: vec![vec![]] };
    let at = span(&sources, function.body.expressions[2].span);

    for mode in 0..7 {
        let mut errors = Errors::new(&sources);
        let mut cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry");
        let value_base =
            zryna_ir::data_ownership_v1::MAX_VALUES_PER_FUNCTION - usize::from(mode != 1);
        cfg.value_types.resize(value_base, vec_ty.ir);
        cfg.transitions = zryna_ir::data_ownership_v1::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
            - usize::from(mode != 3);
        let place_base =
            zryna_ir::data_ownership_v1::MAX_PLACES_PER_FUNCTION - usize::from(mode != 2);
        let places = (0..place_base)
            .map(|index| raw::Place {
                id: raw::PlaceId(u32::try_from(index).expect("place id")),
                ty: vec_ty.ir,
                span: at,
                kind: raw::PlaceKind::Local(u32::try_from(index).expect("local id")),
            })
            .collect::<Vec<_>>();
        let source_place = raw::PlaceId(0);
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
            element,
            errors: &mut errors,
            bindings: std::collections::BTreeMap::from([(
                "source".to_owned(),
                super::super::Binding { ty: vec_ty, place: source_place, mutable: false },
            )]),
            places,
            reserved_places: 0,
            cfg,
            cleanup_plans: Vec::new(),
            cleanup_actions: 0,
            reserved_cleanup_plans: if mode == 4 {
                zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION
            } else {
                zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION - 1
            },
            reserved_cleanup_actions: if mode == 5 {
                usize::MAX
            } else if mode == 6 {
                zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION
            } else {
                zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION - 1
            },
            owners: OwnerState {
                pending: vec![source_place],
                value_owners: std::collections::BTreeMap::new(),
            },
            known_string_bytes: std::collections::BTreeMap::new(),
            next_value: u32::try_from(value_base).expect("value id"),
            next_local: u32::try_from(place_base).expect("local id"),
        };
        let before = (
            lowerer.places.len(),
            lowerer.cfg.value_types.len(),
            lowerer.cfg.transitions,
            lowerer.cfg.current_block().expect("entry").instructions.clone(),
            lowerer.owners.clone(),
            lowerer.cleanup_plans.clone(),
        );
        let result = lowerer.clone_vec(1, vec_ty, at);
        if mode == 0 {
            assert!(result.is_some(), "exact compound reservation");
            assert!(lowerer.owners.contains(source_place), "clone preserves source");
        } else {
            assert!(result.is_none(), "first extra or overflow must fail");
            assert_eq!(
                (
                    lowerer.places.len(),
                    lowerer.cfg.value_types.len(),
                    lowerer.cfg.transitions,
                    lowerer.cfg.current_block().expect("entry").instructions.clone(),
                    lowerer.owners.clone(),
                    lowerer.cleanup_plans.clone(),
                ),
                before
            );
        }
        drop(lowerer);
        let diagnostics = errors.finish();
        if mode == 0 {
            assert!(diagnostics.is_empty());
        } else {
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
            assert_eq!(diagnostics[0].primary_span(), Some(at));
        }
    }

    for case in 0..3 {
        let mut errors = Errors::new(&sources);
        let cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry");
        let source_place = raw::PlaceId(0);
        let binding_ty = if case == 1 { element } else { vec_ty };
        let owners = if case == 0 {
            OwnerState::default()
        } else {
            OwnerState {
                pending: vec![source_place],
                value_owners: std::collections::BTreeMap::new(),
            }
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
            element,
            errors: &mut errors,
            bindings: std::collections::BTreeMap::from([(
                "source".to_owned(),
                super::super::Binding { ty: binding_ty, place: source_place, mutable: false },
            )]),
            places: vec![raw::Place {
                id: source_place,
                ty: vec_ty.ir,
                span: at,
                kind: raw::PlaceKind::Local(0),
            }],
            reserved_places: 0,
            cfg,
            cleanup_plans: Vec::new(),
            cleanup_actions: 0,
            reserved_cleanup_plans: 0,
            reserved_cleanup_actions: 0,
            owners,
            known_string_bytes: std::collections::BTreeMap::new(),
            next_value: 0,
            next_local: 1,
        };
        let operand = u32::from(case != 2);
        assert!(lowerer.clone_vec(operand, vec_ty, at).is_none(), "negative case {case}");
        assert!(lowerer.cfg.current_block().expect("entry").instructions.is_empty());
        assert_eq!(lowerer.places.len(), 1);
        assert!(lowerer.cleanup_plans.is_empty());
        drop(lowerer);
        let diagnostics = errors.finish();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code(), if case == 0 { "ZRYNA-M3014" } else { "ZRYNA-M3013" });
        let expected = if case == 2 {
            span(&sources, function.body.expressions[0].span)
        } else {
            span(&sources, function.body.expressions[1].span)
        };
        assert_eq!(diagnostics[0].primary_span(), Some(expected));
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn private_vec_string_clone_prefix_cleanup_is_exact_plus_one_and_overflow_atomic() {
    let (source, raw) = private_vec_clone_fixture("String");
    let sources = sources_for(&source);
    let syntax = verify_snapshot(raw, &sources).expect("source-faithful Vec<String> clone");
    let input = pair_input(&syntax, &sources);
    let mut setup_errors = Errors::new(&sources);
    semantic_preflight(input, &mut setup_errors);
    let (graph, declarations) = super::super::build_graph(input, &mut setup_errors);
    let layouts = zryna_layout::verify(&graph, &sources, zryna_layout::StorageTarget::Linear32V1)
        .expect("verified layouts");
    let node_types = super::super::map_node_types(&graph, &layouts, &mut setup_errors);
    assert!(setup_errors.finish().is_empty());
    let vec_ty = node_types
        .iter()
        .flatten()
        .find(|ty| ty.category == zryna_layout::TypeCategory::Vec)
        .copied()
        .expect("Vec<String>");
    let element = node_types
        .iter()
        .flatten()
        .find(|ty| ty.category == zryna_layout::TypeCategory::String)
        .copied()
        .expect("String");
    let file = &syntax.files()[0];
    let function = &file.functions()[0];
    let catalog = FunctionCatalog { modules: vec![vec![]] };
    let at = span(&sources, function.body.expressions[5].span);

    for mode in 0..4 {
        let mut errors = Errors::new(&sources);
        let cfg = OwnedCfgState::single_block(at, &mut errors).expect("entry");
        let source_place = raw::PlaceId(0);
        let owners = OwnerState {
            pending: vec![source_place],
            value_owners: std::collections::BTreeMap::new(),
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
            element,
            errors: &mut errors,
            bindings: std::collections::BTreeMap::from([(
                "source".to_owned(),
                super::super::Binding { ty: vec_ty, place: source_place, mutable: false },
            )]),
            places: vec![raw::Place {
                id: source_place,
                ty: vec_ty.ir,
                span: at,
                kind: raw::PlaceKind::Local(0),
            }],
            reserved_places: 0,
            cfg,
            cleanup_plans: Vec::new(),
            cleanup_actions: 0,
            reserved_cleanup_plans: if mode == 1 {
                zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION - 1
            } else {
                zryna_ir::data_ownership_v1::MAX_CLEANUP_PLANS_PER_FUNCTION - 2
            },
            reserved_cleanup_actions: match mode {
                2 => zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION - 2,
                3 => usize::MAX,
                _ => zryna_ir::data_ownership_v1::MAX_DROP_ACTIONS_PER_FUNCTION - 3,
            },
            owners,
            known_string_bytes: std::collections::BTreeMap::new(),
            next_value: 0,
            next_local: 1,
        };
        let before = (
            lowerer.places.clone(),
            lowerer.cfg.current_block().expect("entry").instructions.clone(),
            lowerer.owners.clone(),
            lowerer.cleanup_plans.clone(),
            lowerer.next_value,
        );
        let result = lowerer.clone_vec(4, vec_ty, at);
        if mode == 0 {
            assert!(result.is_some(), "exact two-phase cleanup budget");
            assert_eq!(lowerer.cleanup_plans.len(), 2);
            assert_eq!(lowerer.cleanup_actions, 3);
        } else {
            assert!(result.is_none(), "first extra or overflow must fail");
            assert_eq!(
                (
                    lowerer.places.clone(),
                    lowerer.cfg.current_block().expect("entry").instructions.clone(),
                    lowerer.owners.clone(),
                    lowerer.cleanup_plans.clone(),
                    lowerer.next_value,
                ),
                before
            );
        }
        drop(lowerer);
        let diagnostics = errors.finish();
        if mode == 0 {
            assert!(diagnostics.is_empty());
        } else {
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(diagnostics[0].code(), "ZRYNA-M3201");
            assert_eq!(diagnostics[0].primary_span(), Some(at));
        }
    }
}
