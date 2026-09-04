use super::*;
use crate::data_ownership_v1::layout_graph::semantic_type;
use crate::data_ownership_v1::owned_constructor_plan::ConstructorValueTypes;
use crate::data_ownership_v1::tests::constructor_envelope_fixtures::{self as fixtures, Fixture};
use crate::data_ownership_v1::{self as ownership, Errors, OwnerState, span};
use std::collections::{BTreeMap, BTreeSet};
use zryna_diagnostics::Diagnostic;
use zryna_syntax::v4::{
    RawExpressionKind, RawFieldInitializerKind, RawProjectSyntaxSnapshot, RawStatementKind,
    verify_snapshot,
};

#[path = "aggregate_constructor_envelope_flows.rs"]
mod flows;

#[path = "aggregate_constructor_projection_integration.rs"]
mod integration;

#[path = "ordered_expression_decisions.rs"]
mod decisions;

fn with_snapshot(
    source: &str,
    snapshot: RawProjectSyntaxSnapshot,
    exercise: impl FnOnce(&mut PrivateOwnedAggregateLowerer<'_, '_, '_>, Ty),
) -> Vec<Diagnostic> {
    let sources = fixtures::sources(source);
    let syntax = verify_snapshot(snapshot, &sources).expect("authenticated source fixture");
    let input = fixtures::input(&syntax, &sources);
    let mut errors = Errors::new(&sources);
    ownership::semantic_preflight(input, &mut errors);
    let (graph, declarations) = ownership::build_graph(input, &mut errors);
    let layouts = zryna_layout::verify(&graph, &sources, zryna_layout::StorageTarget::Linear32V1)
        .expect("authenticated layouts");
    let node_types = ownership::map_node_types(&graph, &layouts, &mut errors);
    let file = &syntax.files()[0];
    let function = &file.functions()[0];
    let result = semantic_type(
        file,
        function.result_type,
        0,
        &declarations,
        &graph,
        &node_types,
        &mut errors,
    )
    .expect("exact result type");
    assert!(errors.is_empty());
    let mut lowerer = PrivateOwnedAggregateLowerer {
        input,
        file,
        function,
        module: 0,
        declarations: &declarations,
        graph: &graph,
        node_types: &node_types,
        layouts: &layouts,
        errors: &mut errors,
        bindings: BTreeMap::new(),
        projections: BTreeMap::new(),
        moved_projections: BTreeSet::new(),
        partial_roots: BTreeSet::new(),
        places: vec![],
        instructions: vec![],
        constructor_types: ConstructorValueTypes::default(),
        constructor_storage: ConstructorStorage::default(),
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

fn with_fixture(
    fixture: Fixture,
    exercise: impl FnOnce(&mut PrivateOwnedAggregateLowerer<'_, '_, '_>, Ty),
) -> Vec<Diagnostic> {
    let (source, snapshot) = fixtures::snapshot(fixture);
    with_snapshot(&source, snapshot, exercise)
}

fn credits(lowerer: &PrivateOwnedAggregateLowerer<'_, '_, '_>) -> [usize; 4] {
    [
        lowerer.constructor_storage.operands,
        lowerer.reserved_transitions,
        lowerer.constructor_storage.values,
        lowerer.constructor_storage.places,
    ]
}

fn set_credits(lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>, values: [usize; 4]) {
    lowerer.constructor_storage =
        ConstructorStorage { operands: values[0], values: values[2], places: values[3] };
    lowerer.reserved_transitions = values[1];
}

fn root_value(lowerer: &PrivateOwnedAggregateLowerer<'_, '_, '_>, statement: usize) -> u32 {
    match lowerer.function.body.statements[statement].kind {
        RawStatementKind::LocalDeclaration { initializer, .. } => initializer,
        RawStatementKind::Return { value, .. } => value,
        _ => panic!("fixture expression statement"),
    }
}

fn run_statement(
    lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>,
    index: usize,
    result: Ty,
) -> bool {
    let body = &lowerer.function.body;
    let final_statement = body.blocks[0].statements.last().copied();
    let statement = body.statements[index].clone();
    lowerer
        .lower_statement(
            u32::try_from(index).expect("fixture statement"),
            &statement,
            result,
            final_statement,
            1,
        )
        .is_some()
}

const LIMITS: [usize; 4] = [
    ir::MAX_AGGREGATE_OPERANDS,
    ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION,
    ir::MAX_VALUES_PER_FUNCTION,
    ir::MAX_PLACES_PER_FUNCTION,
];

fn assert_diagnostic(errors: &[Diagnostic], message: &str) {
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code(), "ZRYNA-M3201");
    assert_eq!(errors[0].message(), message);
}

#[test]
fn constructor_envelope_exact_credit_acquisition_overflow_and_reverse_release_are_atomic() {
    let errors = with_fixture(Fixture::Pair, |lowerer, result| {
        let at = span(lowerer.input.sources(), lowerer.function.span);
        let exact = [LIMITS[0] - 2, LIMITS[1] - 1, LIMITS[2] - 1, LIMITS[3] - 1];
        set_credits(lowerer, exact);
        let ticket = lowerer.reserve_constructor_commit(result, 2, at).expect("exact envelope");
        assert_eq!(credits(lowerer), LIMITS);
        ticket.release(lowerer);
        assert_eq!(credits(lowerer), exact);
        assert!(lowerer.instructions.is_empty());
        assert!(lowerer.places.is_empty());
        assert!(lowerer.cleanup_plans.is_empty(), "infallible commit has no cleanup ticket");
    });
    assert!(errors.is_empty());
    for dimension in 0..4 {
        for overflow in [false, true] {
            let errors = with_fixture(Fixture::Pair, |lowerer, result| {
                let mut held = [0; 4];
                held[dimension] = if overflow { usize::MAX } else { LIMITS[dimension] };
                set_credits(lowerer, held);
                let at = span(lowerer.input.sources(), lowerer.function.span);
                assert!(lowerer.reserve_constructor_commit(result, 2, at).is_none());
                assert_eq!(credits(lowerer), held, "failed acquisition is atomic");
                assert!(lowerer.instructions.is_empty());
                assert!(lowerer.owners.pending().is_empty());
            });
            assert_eq!(errors.len(), 1);
            assert_eq!(errors[0].code(), "ZRYNA-M3201");
        }
    }
}

#[test]
fn constructor_envelope_competing_capacity_errors_precede_children_but_follow_outer_mapping() {
    for dimension in 0..5 {
        let (mut source, mut snapshot) = fixtures::snapshot(Fixture::Pair);
        let expression = &mut snapshot.files[0].functions[0].body.expressions[1];
        let child_span = expression.span;
        source.replace_range(child_span.start as usize..child_span.end as usize, "bad");
        expression.kind = RawExpressionKind::Reference {
            name: zryna_syntax::v4::RawIdentifierSyntax {
                text: "bad".to_owned(),
                span: child_span,
            },
        };
        for _ in 0..2 {
            let errors = with_snapshot(&source, snapshot.clone(), |lowerer, result| {
                let mut held = LIMITS;
                held[..dimension.min(4)].fill(0);
                set_credits(lowerer, held);
                let expression = root_value(lowerer, 0);
                assert!(lowerer.value(expression, result).is_none());
                assert_eq!(credits(lowerer), held);
                assert!(lowerer.instructions.is_empty());
                assert!(lowerer.cleanup_plans.is_empty());
            });
            assert_eq!(errors.len(), 1);
            if dimension == 4 {
                assert_eq!(errors[0].code(), "ZRYNA-M3002");
                assert_eq!(errors[0].message(), "aggregate value 'bad' is not declared");
                assert_eq!(
                    errors[0].primary_span().map(|at| (at.start(), at.end())),
                    Some((child_span.start, child_span.end))
                );
            } else {
                let messages = [
                    format!("derived aggregate operands exceed the M3 limit of {}", LIMITS[0]),
                    format!(
                        "derived ownership transitions exceed the per-function M3 limit of {}",
                        LIMITS[1]
                    ),
                    format!("derived values exceed the per-function M3 limit of {}", LIMITS[2]),
                    format!("derived places exceed the per-function M3 limit of {}", LIMITS[3]),
                ];
                assert_diagnostic(&errors, &messages[dimension]);
                assert_eq!(
                    errors[0].primary_span().map(|at| (at.start(), at.end())),
                    Some((121, 158))
                );
            }
        }
    }
    let (mut source, mut snapshot) = fixtures::snapshot(Fixture::Pair);
    let RawExpressionKind::StructConstruction { fields, .. } =
        &mut snapshot.files[0].functions[0].body.expressions[2].kind
    else {
        panic!("struct")
    };
    let RawFieldInitializerKind::Explicit { name, .. } = &mut fields[1].kind else {
        panic!("field")
    };
    source.replace_range(name.span.start as usize..name.span.end as usize, "other");
    name.text = "other".to_owned();
    let errors = with_snapshot(&source, snapshot, |lowerer, result| {
        set_credits(lowerer, LIMITS);
        assert!(lowerer.value(root_value(lowerer, 0), result).is_none());
        assert_eq!(credits(lowerer), LIMITS);
    });
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code(), "ZRYNA-M3016");
    assert_eq!(errors[0].message(), "struct 'OwnedPair' has no field 'other'");
}

#[test]
fn constructor_envelope_nested_exact_and_first_extra_restore_all_surrounding_credits() {
    let costs = [3, 4, 4, 4];
    for failed_dimension in 0..5 {
        let errors = with_fixture(Fixture::Nested, |lowerer, result| {
            let mut held = std::array::from_fn(|index| LIMITS[index] - costs[index]);
            if failed_dimension < 4 {
                held[failed_dimension] += 1;
            }
            set_credits(lowerer, held);
            let value = lowerer.value(root_value(lowerer, 0), result);
            assert_eq!(credits(lowerer), held);
            if failed_dimension == 4 {
                let value = value.expect("exact nested constructor frontier");
                assert_eq!(lowerer.instructions.len(), 4);
                assert_eq!(lowerer.places.len(), 4);
                assert_eq!(lowerer.aggregate_operands, 3);
                assert_eq!(
                    lowerer.owners.pending(),
                    &[lowerer.owners.owner(value).expect("result")]
                );
            } else {
                assert!(value.is_none());
            }
            assert!(lowerer.budget_values() <= LIMITS[2]);
            assert!(lowerer.budget_places() <= LIMITS[3]);
            assert!(lowerer.budget_transitions() <= LIMITS[1]);
            assert!(lowerer.budget_operands() <= LIMITS[0]);
        });
        assert_eq!(errors.len(), usize::from(failed_dimension < 4));
    }
}

#[test]
fn constructor_envelope_valid_source_replay_retains_independent_full_ir_authority() {
    for fixture in [
        Fixture::Pair,
        Fixture::Nested,
        Fixture::Array,
        Fixture::Enum,
        Fixture::EmptyEnum,
        Fixture::WholeClone,
        Fixture::Projection,
        Fixture::PartialTransfer,
    ] {
        let (source, snapshot) = fixtures::snapshot(fixture);
        let sources = fixtures::sources(&source);
        let syntax = verify_snapshot(snapshot, &sources).expect("source-faithful fixture");
        let mut previous = None;
        for _ in 0..2 {
            let program = ownership::lower(fixtures::input(&syntax, &sources))
                .expect("mandatory independent full IR verification");
            let kinds = program
                .modules()
                .flat_map(zryna_ir::data_ownership_v1::VerifiedModule::functions)
                .flat_map(zryna_ir::data_ownership_v1::VerifiedFunction::blocks)
                .flat_map(zryna_ir::data_ownership_v1::VerifiedBlock::instructions)
                .map(zryna_ir::data_ownership_v1::VerifiedInstruction::kind)
                .collect::<Vec<_>>();
            if let Some(previous) = &previous {
                assert_eq!(&kinds, previous);
            }
            previous = Some(kinds);
        }
    }
}
