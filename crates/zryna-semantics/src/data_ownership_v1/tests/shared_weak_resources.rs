use super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::constructor_resources::tests::{run_statement, with_snapshot};
use super::*;
use crate::data_ownership_v1::layout_graph::semantic_type;
use crate::data_ownership_v1::tests::generic_vec_fixture::shared_weak_fixture::{
    Case, fixture, fixture_case,
};
use crate::data_ownership_v1::{Binding, span};
use zryna_diagnostics::Diagnostic;
use zryna_ir::data_ownership_v1 as ir;
use zryna_syntax::v4::RawStatementKind;

fn register_parameter(lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>) {
    let parameter = &lowerer.function.parameters[0];
    let ty = semantic_type(
        lowerer.file,
        parameter.type_syntax,
        lowerer.module,
        lowerer.declarations,
        lowerer.graph,
        lowerer.node_types,
        lowerer.errors,
    )
    .expect("authenticated payload parameter");
    let value = raw::ValueId(0);
    let at = span(lowerer.input.sources(), parameter.span);
    lowerer.next_value = 1;
    lowerer
        .constructor_types
        .record_parameter(&raw::ValueDefinition { id: value, ty: ty.ir, span: at })
        .expect("dense parameter identity");
    let place = raw::PlaceId(0);
    lowerer.places.push(raw::Place {
        id: place,
        ty: ty.ir,
        span: at,
        kind: raw::PlaceKind::Parameter(0),
    });
    lowerer.bindings.insert(parameter.name.text.clone(), Binding { ty, place, mutable: false });
    if !ty.is_copy() {
        lowerer.owners.register_parameter(place).expect("owned payload parameter");
    }
}

#[test]
fn shared_construct_cleanup_frontier_is_exact_atomic_and_recovers() {
    let (source, snapshot) = fixture();
    for extra in [false, true] {
        let mut expected = None;
        let errors = with_snapshot(&source, snapshot.clone(), |lowerer, result| {
            register_parameter(lowerer);
            let RawStatementKind::LocalDeclaration { initializer, ref name, mutable, .. } =
                lowerer.function.body.statements[0].kind
            else {
                panic!("Shared local declaration");
            };
            let at = span(lowerer.input.sources(), lowerer.function.body.statements[0].span);
            lowerer.cleanup_actions = ir::MAX_DROP_ACTIONS_PER_FUNCTION - 1 + usize::from(extra);
            let before = state(lowerer);
            let checkpoint = lowerer.preparation_checkpoint();
            let facts = lowerer.preparation_facts.clone();
            if extra {
                assert!(
                    PreparedLocal::prepare(lowerer, initializer, result, at, &name.text, mutable)
                        .is_none()
                );
                assert_eq!(state(lowerer), before);
                assert_eq!(lowerer.preparation_checkpoint(), checkpoint);
                assert_eq!(lowerer.preparation_facts, facts);
                expected = Some(Diagnostic::error_at(
                    "ZRYNA-M3201",
                    span(lowerer.input.sources(), lowerer.function.body.expressions[1].span),
                    format!(
                        "derived cleanup actions exceed the per-function M3 limit of {}",
                        ir::MAX_DROP_ACTIONS_PER_FUNCTION
                    ),
                    "reduce simultaneously live owned aggregates and String leaves",
                ));
                lowerer.cleanup_actions = 0;
                PreparedLocal::prepare(lowerer, initializer, result, at, &name.text, mutable)
                    .expect("valid preparation recovers after rejected frontier")
                    .consume();
            } else {
                PreparedLocal::prepare(lowerer, initializer, result, at, &name.text, mutable)
                    .expect("exact cleanup frontier")
                    .consume();
                assert_eq!(lowerer.cleanup_actions, ir::MAX_DROP_ACTIONS_PER_FUNCTION);
            }
            assert!(lowerer.constructor_storage_is_clear());
        });
        assert_eq!(errors, expected.into_iter().collect::<Vec<_>>());
    }
}

#[test]
fn temporary_handle_drop_transition_has_exact_extra_boundary_and_recovers() {
    let (source, snapshot) = fixture_case(Case::TemporaryOperands);
    for extra in [false, true] {
        let mut expected = None;
        let errors = with_snapshot(&source, snapshot.clone(), |lowerer, result| {
            register_parameter(lowerer);
            assert!(run_statement(lowerer, 0, result));
            let initial =
                ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - lowerer.instructions.len() - 4
                    + usize::from(extra);
            lowerer.reserved_transitions = initial;
            let before = state(lowerer);
            let checkpoint = lowerer.preparation_checkpoint();
            let facts = lowerer.preparation_facts.clone();
            let count = lowerer.instructions.len();
            assert_eq!(run_statement(lowerer, 1, result), !extra);
            if extra {
                assert_eq!(state(lowerer), before);
                assert_eq!(lowerer.preparation_checkpoint(), checkpoint);
                assert_eq!(lowerer.preparation_facts, facts);
                expected = Some(Diagnostic::error_at(
                    "ZRYNA-M3201",
                    span(lowerer.input.sources(), lowerer.function.body.statements[1].span),
                    format!(
                        "derived ownership transitions exceed the per-function M3 limit of {}",
                        ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
                    ),
                    "reduce private aggregate expressions and assignments",
                ));
                lowerer.reserved_transitions = 0;
                assert!(run_statement(lowerer, 1, result), "valid retry after rejected summary");
            } else {
                assert_eq!(lowerer.instructions.len() - count, 4);
            }
            lowerer.reserved_transitions = 0;
            assert!(lowerer.constructor_storage_is_clear());
        });
        assert_eq!(errors, expected.into_iter().collect::<Vec<_>>());
    }
}
