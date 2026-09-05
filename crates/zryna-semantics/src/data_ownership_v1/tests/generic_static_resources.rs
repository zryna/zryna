use super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::constructor_resources::tests::{run_statement, with_snapshot};
use super::*;
use crate::data_ownership_v1::layout_graph::semantic_type;
use crate::data_ownership_v1::tests::generic_static_fixture::{Case, Shape, fixture};
use crate::data_ownership_v1::type_model::Binding;
use zryna_ir::data_ownership_v1 as ir;
use zryna_syntax::v4::RawStatementKind;

fn parameters(lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>) {
    for (index, parameter) in lowerer.function.parameters.iter().enumerate() {
        let ty = semantic_type(
            lowerer.file,
            parameter.type_syntax,
            lowerer.module,
            lowerer.declarations,
            lowerer.graph,
            lowerer.node_types,
            lowerer.errors,
        )
        .expect("parameter type");
        let at = crate::data_ownership_v1::span(lowerer.input.sources(), parameter.span);
        let id = u32::try_from(index).expect("parameter");
        let value = raw::ValueDefinition { id: raw::ValueId(id), ty: ty.ir, span: at };
        lowerer.constructor_types.record_parameter(&value).expect("dense parameters");
        let place = raw::PlaceId(id);
        lowerer.places.push(raw::Place {
            id: place,
            ty: ty.ir,
            span: at,
            kind: raw::PlaceKind::Parameter(id),
        });
        lowerer.bindings.insert(parameter.name.text.clone(), Binding { ty, place, mutable: false });
        lowerer.owners.register_parameter(place).expect("owned parameter");
        lowerer.next_value += 1;
    }
    // The envelope helper selects its route from the result alone. These fixtures
    // select the generic driver through their mixed aggregate input parameter.
    assert!(lowerer.bindings.values().any(|binding| {
        super::super::mixed_shape::requires_summary(binding.ty, lowerer.layouts)
    }));
    lowerer.mixed_function = true;
}

#[test]
fn generic_static_replacement_resources_exact_extra_overflow_and_recovery_are_transactional() {
    for shape in [Shape::Struct, Shape::Array, Shape::ArrayStruct] {
        for pressure in [0, 1, 2, 0] {
            let (source, snapshot) = fixture(shape, Case::Replace);
            let mut expected = None;
            let errors = with_snapshot(&source, snapshot, |lowerer, root_ty| {
                parameters(lowerer);
                assert!(run_statement(lowerer, 0, root_ty));
                let statement = &lowerer.function.body.statements[1];
                let RawStatementKind::Assignment { target, value, .. } = statement.kind else {
                    panic!("replacement");
                };
                let at = crate::data_ownership_v1::span(lowerer.input.sources(), statement.span);
                let ty = lowerer.projection_expression_type(target).expect("exact static type");
                let pending = lowerer.owners.pending().to_vec();
                let required = 2 * pending.len() + 1;
                let held = if pressure == 2 {
                    usize::MAX
                } else {
                    ir::MAX_DROP_ACTIONS_PER_FUNCTION - lowerer.cleanup_actions - required
                        + pressure
                };
                lowerer.preparation_facts.held_cleanup[1] = held;
                let before = state(lowerer);
                let checkpoint = lowerer.preparation_checkpoint();
                let facts = lowerer.preparation_facts.clone();
                let plans = lowerer.cleanup_plans.len();
                let transitions = lowerer.instructions.len();
                if pressure == 0 {
                    let prepared =
                        PreparedValue::prepare_static_replacement(lowerer, target, value, ty, at)
                            .expect("exact replacement limit");
                    assert_eq!(state(prepared.lowerer), before);
                    assert_eq!(prepared.lowerer.preparation_checkpoint(), checkpoint);
                    prepared.consume();
                    assert_eq!(lowerer.cleanup_actions + held, ir::MAX_DROP_ACTIONS_PER_FUNCTION);
                    assert_eq!(lowerer.cleanup_plans.len() - plans, 2);
                    assert_eq!(lowerer.instructions.len() - transitions, 2);
                    assert_eq!(lowerer.owners.pending(), pending.as_slice());
                    assert!(lowerer.moved_projections.is_empty());
                    assert!(lowerer.partial_roots.is_empty());
                    assert!(matches!(
                        lowerer.instructions.last().expect("commit").kind,
                        raw::InstructionKind::GenericReplacePlace { .. }
                    ));
                } else {
                    assert!(
                        PreparedValue::prepare_static_replacement(lowerer, target, value, ty, at)
                            .is_none()
                    );
                    assert_eq!(state(lowerer), before);
                    assert_eq!(lowerer.preparation_checkpoint(), checkpoint);
                    assert_eq!(lowerer.preparation_facts, facts);
                    let clone_at = lowerer.function.body.expressions[value as usize].span;
                    expected = Some(zryna_diagnostics::Diagnostic::error_at(
                        "ZRYNA-M3201",
                        crate::data_ownership_v1::span(lowerer.input.sources(), clone_at),
                        "structural clone exceeds a checked value, place, or cleanup resource limit",
                        "reduce simultaneously live owned aggregates or clone sites",
                    ));
                }
                assert_eq!(lowerer.preparation_facts.held_cleanup[1], held);
                lowerer.preparation_facts.held_cleanup[1] = 0;
                if pressure != 0 {
                    PreparedValue::prepare_static_replacement(lowerer, target, value, ty, at)
                        .expect("same lowerer recovers after rejected reservation")
                        .consume();
                    assert_eq!(lowerer.owners.pending(), pending.as_slice());
                    assert!(lowerer.moved_projections.is_empty());
                }
                assert!(lowerer.constructor_storage_is_clear());
            });
            assert_eq!(
                errors,
                expected.into_iter().collect::<Vec<_>>(),
                "{shape:?} pressure={pressure}"
            );
        }
    }
}

#[test]
fn generic_static_move_resources_exact_extra_overflow_and_recovery_do_not_leak_masks() {
    for shape in [Shape::Struct, Shape::Array, Shape::ArrayStruct] {
        for pressure in [0, 1, 2, 0] {
            let (source, snapshot) = fixture(shape, Case::Move);
            let mut expected = None;
            let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
                parameters(lowerer);
                assert!(run_statement(lowerer, 0, ty));
                let RawStatementKind::Return { value, .. } =
                    lowerer.function.body.statements[1].kind
                else {
                    panic!("return");
                };
                let held = if pressure == 2 {
                    usize::MAX
                } else {
                    ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - lowerer.instructions.len() - 1
                        + pressure
                };
                lowerer.reserved_transitions = held;
                let before = state(lowerer);
                let checkpoint = lowerer.preparation_checkpoint();
                let facts = lowerer.preparation_facts.clone();
                let pending = lowerer.owners.pending().len();
                if pressure == 0 {
                    let prepared = PreparedValue::prepare(lowerer, value, ty)
                        .expect("exact move transition limit");
                    assert_eq!(state(prepared.lowerer), before);
                    assert_eq!(prepared.lowerer.preparation_checkpoint(), checkpoint);
                    prepared.consume();
                    assert_eq!(
                        lowerer.instructions.len() + held,
                        ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
                    );
                    assert_eq!(lowerer.owners.pending().len(), pending + 1);
                    assert_eq!(lowerer.moved_projections.len(), 1);
                    assert_eq!(lowerer.partial_roots.len(), 1);
                    assert!(matches!(
                        lowerer.instructions.last().expect("move").kind,
                        raw::InstructionKind::GenericMoveFromPlace { .. }
                    ));
                } else {
                    assert!(PreparedValue::prepare(lowerer, value, ty).is_none());
                    assert_eq!(state(lowerer), before);
                    assert_eq!(lowerer.preparation_checkpoint(), checkpoint);
                    assert_eq!(lowerer.preparation_facts, facts);
                    let at = lowerer.function.body.expressions[value as usize].span;
                    expected = Some(zryna_diagnostics::Diagnostic::error_at(
                        "ZRYNA-M3201",
                        crate::data_ownership_v1::span(lowerer.input.sources(), at),
                        format!(
                            "derived ownership transitions exceed the per-function M3 limit of {}",
                            ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION
                        ),
                        "reduce private aggregate expressions and assignments",
                    ));
                }
                assert_eq!(lowerer.reserved_transitions, held);
                lowerer.reserved_transitions = 0;
                if pressure != 0 {
                    PreparedValue::prepare(lowerer, value, ty)
                        .expect("same lowerer recovers after rejected transition reservation")
                        .consume();
                    assert_eq!(lowerer.moved_projections.len(), 1);
                    assert_eq!(lowerer.owners.pending().len(), pending + 1);
                }
                assert!(lowerer.constructor_storage_is_clear());
            });
            assert_eq!(
                errors,
                expected.into_iter().collect::<Vec<_>>(),
                "{shape:?} pressure={pressure}"
            );
        }
    }
}
