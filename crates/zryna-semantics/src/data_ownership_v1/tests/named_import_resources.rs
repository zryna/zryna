use super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::constructor_resources::tests::{root_value, run_statement};
use super::cleanup_frontiers::seed_external;
use super::*;
use crate::data_ownership_v1::function_catalog::build_function_catalog;
use crate::data_ownership_v1::import_resolution;
use crate::data_ownership_v1::layout_graph::{build_graph, semantic_type};
use crate::data_ownership_v1::owned_constructor_plan::ConstructorValueTypes;
use crate::data_ownership_v1::string_vec_resource_estimates::owned_call_cleanup_budget_violation;
use crate::data_ownership_v1::tests::named_import_calls::imported_zero_argument_fixture;
use crate::data_ownership_v1::type_model::map_node_types;
use crate::data_ownership_v1::{Errors, OwnerState, SemanticInput, semantic_preflight};
use std::collections::{BTreeMap, BTreeSet};
use zryna_ir::data_ownership_v1 as ir;
use zryna_source::NormalizedSourcePath;
use zryna_syntax::v4::verify_snapshot;

fn with_imported(
    exercise: impl FnOnce(&mut PrivateOwnedAggregateLowerer<'_, '_, '_>, Ty),
) -> Vec<zryna_diagnostics::Diagnostic> {
    let (sources, raw) = imported_zero_argument_fixture();
    let syntax = verify_snapshot(raw, &sources).expect("authenticated imported source");
    let entry = sources.file_id(&NormalizedSourcePath::new("src/main.zry").unwrap()).unwrap();
    let input = SemanticInput::try_new(&syntax, &sources, entry).unwrap();
    let mut errors = Errors::new(&sources);
    semantic_preflight(input, &mut errors);
    let (graph, declarations) = build_graph(input, &mut errors);
    let layouts = zryna_layout::verify(&graph, &sources, zryna_layout::StorageTarget::Linear32V1)
        .expect("layouts");
    let node_types = map_node_types(&graph, &layouts, &mut errors);
    let mut catalog =
        build_function_catalog(input, &declarations, &graph, &node_types, &mut errors);
    import_resolution::resolve_imports(input, &mut catalog, &mut errors);
    let module = usize::try_from(entry.index()).unwrap();
    let file = &syntax.files()[module];
    let function = &file.functions()[0];
    let result = semantic_type(
        file,
        function.result_type,
        module,
        &declarations,
        &graph,
        &node_types,
        &mut errors,
    )
    .expect("result type");
    assert!(errors.is_empty());
    let mut lowerer = PrivateOwnedAggregateLowerer {
        input,
        file,
        function,
        module,
        declarations: &declarations,
        graph: &graph,
        node_types: &node_types,
        layouts: &layouts,
        catalog: &catalog,
        mixed_function: true,
        errors: &mut errors,
        bindings: BTreeMap::new(),
        projections: BTreeMap::new(),
        moved_projections: BTreeSet::new(),
        partial_roots: BTreeSet::new(),
        places: vec![],
        instructions: vec![],
        constructor_types: ConstructorValueTypes::default(),
        constructor_storage: Default::default(),
        preparation_facts: super::super::preparation_plan::PreparationFacts::default(),
        cleanup_plans: vec![],
        cleanup_actions: 0,
        aggregate_operands: 0,
        aggregate_subobject_moves: 0,
        projected_aggregate_clones: 0,
        projected_aggregate_assignments: 0,
        reserved_transitions: 0,
        owners: OwnerState::default(),
        next_value: 0,
        next_local: 0,
    };
    exercise(&mut lowerer, result);
    errors.finish()
}

#[test]
fn named_import_preparation_resources_are_exact_atomic_overflow_checked_and_recoverable() {
    for extra in [false, true] {
        let errors = with_imported(|lowerer, ty| {
            assert!(run_statement(lowerer, 0, ty));
            let root = root_value(lowerer, 1);
            let held = ir::MAX_VALUES_PER_FUNCTION - 2 + usize::from(extra);
            let tickets = (0..held)
                .map(|_| lowerer.credit_ledger().acquire_constructor(0, 0).unwrap())
                .collect::<Vec<_>>();
            let before = state(lowerer);
            let facts = lowerer.preparation_facts.clone();
            if extra {
                assert!(PreparedValue::prepare(lowerer, root, ty).is_none());
                assert_eq!(state(lowerer), before);
                assert_eq!(lowerer.preparation_facts, facts);
            } else {
                PreparedValue::prepare(lowerer, root, ty).expect("exact frontier").consume();
            }
            for ticket in tickets.into_iter().rev() {
                ticket.release(lowerer);
            }
            if extra {
                PreparedValue::prepare(lowerer, root, ty)
                    .expect("same authenticated imported call recovers")
                    .consume();
            }
            assert!(lowerer.constructor_storage_is_clear());
        });
        assert_eq!(errors.len(), usize::from(extra));
        if extra {
            assert_eq!(errors[0].code, "ZRYNA-M3201");
        }
    }

    assert!(!owned_call_cleanup_budget_violation(
        ir::MAX_CLEANUP_PLANS_PER_FUNCTION - 1,
        ir::MAX_DROP_ACTIONS_PER_FUNCTION - 1,
        2,
        1,
    ));
    assert!(owned_call_cleanup_budget_violation(0, usize::MAX, 2, 1));

    for extra in [false, true] {
        let errors = with_imported(|lowerer, ty| {
            assert!(run_statement(lowerer, 0, ty));
            let root = root_value(lowerer, 1);
            seed_external(lowerer, ir::MAX_CLEANUP_PLANS_PER_FUNCTION - 2 + usize::from(extra), 0);
            let before = state(lowerer);
            if extra {
                assert!(PreparedValue::prepare(lowerer, root, ty).is_none());
                assert_eq!(state(lowerer), before);
            } else {
                PreparedValue::prepare(lowerer, root, ty)
                    .expect("exact imported cleanup frontier")
                    .consume();
            }
        });
        assert_eq!(errors.len(), usize::from(extra));
        if extra {
            assert_eq!(errors[0].code, "ZRYNA-M3201");
        }
    }
}
