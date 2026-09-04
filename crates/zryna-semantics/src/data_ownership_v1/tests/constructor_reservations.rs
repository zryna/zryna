use super::super::owned_vec_lowering::PrivateVecLowerer;
use super::*;
use zryna_ir::data_ownership_v1 as ir;

#[path = "vec_order_controls.rs"]
mod vec_order_controls;

fn credits(lowerer: &PrivateVecLowerer<'_, '_, '_>) -> [usize; 5] {
    [
        lowerer.cfg.reserved_values,
        lowerer.reserved_places,
        lowerer.cfg.reserved_transitions,
        lowerer.reserved_cleanup_plans,
        lowerer.reserved_cleanup_actions,
    ]
}

fn exercise(lowerer: &mut PrivateVecLowerer<'_, '_, '_>, mode: usize) {
    lowerer.cfg.reserved_values = 2;
    lowerer.reserved_places = 2;
    lowerer.cfg.reserved_transitions = 2;
    lowerer.reserved_cleanup_plans = 2;
    lowerer.reserved_cleanup_actions = 2;
    match mode {
        1 => lowerer.cfg.reserved_values = ir::MAX_VALUES_PER_FUNCTION,
        2 => lowerer.reserved_places = ir::MAX_PLACES_PER_FUNCTION,
        3 => lowerer.cfg.reserved_transitions = ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION,
        4 => lowerer.reserved_cleanup_plans = ir::MAX_CLEANUP_PLANS_PER_FUNCTION,
        5 => lowerer.reserved_cleanup_actions = ir::MAX_DROP_ACTIONS_PER_FUNCTION + 1,
        6 => {
            lowerer.element = lowerer
                .node_types
                .iter()
                .flatten()
                .find(|ty| ty.category == zryna_layout::TypeCategory::Bool)
                .copied()
                .expect("verified constructor fixture");
        }
        7 => lowerer.cfg.reserved_values = ir::MAX_VALUES_PER_FUNCTION - 2,
        _ => {}
    }
    let before = credits(lowerer);
    let result = lowerer.value(2, lowerer.vec_ty);
    assert_eq!(credits(lowerer), before, "reservation release mode {mode}");
    if mode == 0 {
        let result = result.expect("constructor succeeds with surrounding reservations");
        assert_eq!(
            lowerer.owners.pending(),
            &[lowerer.owners.owner(result).expect("verified constructor fixture")]
        );
        assert_eq!(
            lowerer.cfg.current_block().expect("verified constructor fixture").instructions.len(),
            3
        );
        assert_eq!(lowerer.cleanup_plans.len(), 1);
        assert!(lowerer.cleanup_plans[0].actions.is_empty());
    } else {
        assert!(result.is_none(), "first-extra or child failure mode {mode}");
        assert_eq!(
            lowerer.cfg.current_block().expect("verified constructor fixture").instructions.len(),
            usize::from(mode == 7)
        );
        assert!(lowerer.owners.pending().is_empty());
        assert!(lowerer.places.is_empty());
        assert!(lowerer.known_string_bytes.is_empty());
        assert!(lowerer.cleanup_plans.is_empty());
    }
}

#[test]
fn constructor_vec_reservations_unwind_each_failure_and_preserve_surrounding_credits() {
    let sources = sources_for(VEC_INDEX_SOURCE);
    let response = VEC_INDEX_RESPONSE.replacen(
        "\"end\":54}}},{\"span\":{\"file\":0,\"start\":47",
        "\"end\":54}}}},{\"span\":{\"file\":0,\"start\":47",
        1,
    );
    let syntax = verify_snapshot(response_snapshot(&response), &sources)
        .expect("verified constructor fixture");
    let input = pair_input(&syntax, &sources);
    let mut setup_errors = Errors::new(&sources);
    semantic_preflight(input, &mut setup_errors);
    let (graph, declarations) = super::super::build_graph(input, &mut setup_errors);
    let layouts = zryna_layout::verify(&graph, &sources, zryna_layout::StorageTarget::Linear32V1)
        .expect("verified constructor fixture");
    let node_types = super::super::map_node_types(&graph, &layouts, &mut setup_errors);
    assert!(setup_errors.finish().is_empty());
    let vec_ty = node_types
        .iter()
        .flatten()
        .find(|ty| ty.category == zryna_layout::TypeCategory::Vec)
        .copied()
        .expect("verified constructor fixture");
    let element = node_types
        .iter()
        .flatten()
        .find(|ty| ty.category == zryna_layout::TypeCategory::I32)
        .copied()
        .expect("verified constructor fixture");
    let file = &syntax.files()[0];
    let function = &file.functions()[0];
    let at = span(&sources, function.body.expressions[2].span);
    let catalog = FunctionCatalog { modules: vec![vec![]] };
    for mode in 0..8 {
        let mut errors = Errors::new(&sources);
        let cfg =
            OwnedCfgState::single_block(at, &mut errors).expect("verified constructor fixture");
        let mut lowerer = PrivateVecLowerer {
            input,
            file,
            function,
            module: 0,
            declarations: &declarations,
            graph: &graph,
            node_types: &node_types,
            layouts: &layouts,
            catalog: &catalog,
            vec_ty,
            element,
            errors: &mut errors,
            bindings: std::collections::BTreeMap::new(),
            places: vec![],
            reserved_places: 0,
            cfg,
            cleanup_plans: vec![],
            cleanup_actions: 0,
            reserved_cleanup_plans: 0,
            reserved_cleanup_actions: 0,
            owners: OwnerState::default(),
            known_string_bytes: std::collections::BTreeMap::new(),
            next_value: 0,
            next_local: 0,
        };
        exercise(&mut lowerer, mode);
        drop(lowerer);
        let diagnostics = errors.finish();
        if mode == 0 {
            assert!(diagnostics.is_empty());
        } else {
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(
                diagnostics[0].code(),
                if mode == 6 { "ZRYNA-M3013" } else { "ZRYNA-M3201" }
            );
            let expected = if mode == 6 {
                function.body.expressions[0].span
            } else if mode == 7 {
                function.body.expressions[1].span
            } else {
                function.body.expressions[2].span
            };
            assert_eq!(diagnostics[0].primary_span(), Some(span(&sources, expected)));
        }
    }
    assert!(lower(input).is_ok(), "valid full verifier replay after rejected resource states");
}
