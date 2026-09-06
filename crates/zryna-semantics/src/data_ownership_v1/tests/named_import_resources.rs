use super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::constructor_resources::tests::{root_value, run_statement};
use super::cleanup_frontiers::seed_external;
use super::*;
use crate::data_ownership_v1::function_catalog::build_function_catalog;
use crate::data_ownership_v1::import_resolution;
use crate::data_ownership_v1::layout_graph::{build_graph, semantic_type};
use crate::data_ownership_v1::owned_constructor_plan::ConstructorValueTypes;
use crate::data_ownership_v1::string_vec_resource_estimates::owned_call_cleanup_budget_violation;
use crate::data_ownership_v1::tests::named_import_calls::{
    imported_fallible_arguments_fixture, imported_zero_argument_fixture,
};
use crate::data_ownership_v1::type_model::map_node_types;
use crate::data_ownership_v1::{Binding, Errors, OwnerState, SemanticInput, semantic_preflight};
use std::collections::{BTreeMap, BTreeSet};
use zryna_ir::data_ownership_v1 as ir;
use zryna_source::{NormalizedSourcePath, SourceMap};
use zryna_syntax::v4::{RawProjectSyntaxSnapshot, verify_snapshot};

fn with_imported(
    fixture: fn() -> (SourceMap, RawProjectSyntaxSnapshot),
    exercise: impl FnOnce(&mut PrivateOwnedAggregateLowerer<'_, '_, '_>, Ty),
) -> Vec<zryna_diagnostics::Diagnostic> {
    let (sources, raw) = fixture();
    let syntax = verify_snapshot(raw, &sources).expect("authenticated imported source");
    let entry = sources
        .file_id(&NormalizedSourcePath::new("src/main.zry").expect("fixture path"))
        .expect("fixture path");
    let input =
        SemanticInput::try_new(&syntax, &sources, entry).expect("source-bound semantic input");
    let mut errors = Errors::new(&sources);
    semantic_preflight(input, &mut errors);
    let (graph, declarations) = build_graph(input, &mut errors);
    let layouts = zryna_layout::verify(&graph, &sources, zryna_layout::StorageTarget::Linear32V1)
        .expect("layouts");
    let node_types = map_node_types(&graph, &layouts, &mut errors);
    let mut catalog =
        build_function_catalog(input, &declarations, &graph, &node_types, &mut errors);
    import_resolution::resolve_imports(input, &layouts, &mut catalog, &mut errors);
    let module = usize::try_from(entry.index()).expect("bounded fixture index");
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
        constructor_storage: super::super::constructor_resources::ConstructorStorage::default(),
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
    for (index, parameter) in function.parameters.iter().enumerate() {
        let ty = semantic_type(
            file,
            parameter.type_syntax,
            module,
            &declarations,
            &graph,
            &node_types,
            lowerer.errors,
        )
        .expect("authenticated parameter type");
        let value = raw::ValueId(lowerer.next_value);
        lowerer.next_value += 1;
        let definition = raw::ValueDefinition {
            id: value,
            ty: ty.ir,
            span: crate::data_ownership_v1::span(input.sources(), parameter.span),
        };
        lowerer.constructor_types.record_parameter(&definition).expect("dense parameter identity");
        let place =
            raw::PlaceId(u32::try_from(lowerer.places.len()).expect("bounded fixture index"));
        lowerer.places.push(raw::Place {
            id: place,
            ty: ty.ir,
            span: definition.span,
            kind: raw::PlaceKind::Parameter(u32::try_from(index).expect("bounded fixture index")),
        });
        lowerer.bindings.insert(parameter.name.text.clone(), Binding { ty, place, mutable: false });
        if !ty.is_copy() {
            lowerer.owners.register_parameter(place).expect("owned parameter");
        }
    }
    exercise(&mut lowerer, result);
    errors.finish()
}

#[test]
fn named_import_preparation_resources_are_exact_atomic_overflow_checked_and_recoverable() {
    for extra in [false, true] {
        let errors = with_imported(imported_zero_argument_fixture, |lowerer, ty| {
            assert!(run_statement(lowerer, 0, ty));
            let root = root_value(lowerer, 1);
            let held = ir::MAX_VALUES_PER_FUNCTION - 2 + usize::from(extra);
            let tickets = (0..held)
                .map(|_| {
                    lowerer
                        .credit_ledger()
                        .acquire_constructor(0, 0)
                        .expect("checked constructor reservation")
                })
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
        let errors = with_imported(imported_zero_argument_fixture, |lowerer, ty| {
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

#[test]
fn named_import_later_argument_cleanup_action_frontier_is_exact_and_recovers() {
    let mut action_demand = 0;
    let errors = with_imported(imported_fallible_arguments_fixture, |lowerer, ty| {
        assert_eq!(lowerer.next_value, 4, "four authenticated value parameters");
        assert_eq!(lowerer.places.len(), 4, "four addressable parameter places");
        assert_eq!(
            lowerer.owners.pending(),
            [raw::PlaceId(0), raw::PlaceId(2), raw::PlaceId(3)],
            "left, right, and keep are real String owners"
        );
        let root = root_value(lowerer, 0);
        let before_actions = lowerer.cleanup_actions;
        PreparedValue::prepare(lowerer, root, ty).expect("unpressured imported call").consume();
        action_demand = lowerer.cleanup_actions - before_actions;
        assert!(
            action_demand >= 4,
            "cumulative plans include the later producer's earlier literal and three survivors"
        );
    });
    assert!(errors.is_empty());

    for extra in [false, true] {
        let errors = with_imported(imported_fallible_arguments_fixture, |lowerer, ty| {
            assert_eq!(lowerer.next_value, 4);
            assert_eq!(
                lowerer.owners.pending(),
                [raw::PlaceId(0), raw::PlaceId(2), raw::PlaceId(3)]
            );
            let root = root_value(lowerer, 0);
            seed_external(
                lowerer,
                0,
                ir::MAX_DROP_ACTIONS_PER_FUNCTION - action_demand - 3 + usize::from(extra),
            );
            let before = state(lowerer);
            if extra {
                assert!(PreparedValue::prepare(lowerer, root, ty).is_none());
                assert_eq!(state(lowerer), before);
            } else {
                PreparedValue::prepare(lowerer, root, ty)
                    .expect("exact imported later-argument cleanup frontier")
                    .consume();
                assert_eq!(
                    lowerer.cleanup_actions + lowerer.preparation_facts.held_cleanup[1],
                    ir::MAX_DROP_ACTIONS_PER_FUNCTION,
                    "real owner cleanup plus external credits reach the exact final frontier"
                );
            }
        });
        assert_eq!(errors.len(), usize::from(extra));
        if extra {
            assert_eq!(errors[0].code, "ZRYNA-M3201");
        }
    }

    let errors = with_imported(imported_fallible_arguments_fixture, |lowerer, ty| {
        assert_eq!(lowerer.next_value, 4);
        assert_eq!(lowerer.owners.pending(), [raw::PlaceId(0), raw::PlaceId(2), raw::PlaceId(3)]);
        let root = root_value(lowerer, 0);
        PreparedValue::prepare(lowerer, root, ty)
            .expect("pristine imported argument recovery")
            .consume();
        assert!(lowerer.constructor_storage_is_clear());
    });
    assert!(errors.is_empty());
}
