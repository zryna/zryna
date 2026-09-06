use super::super::constructor_resources::tests::with_snapshot;
use super::super::structured_checkpoint::StructuredCheckpoint;
use super::*;
use crate::data_ownership_v1::Binding;
use crate::data_ownership_v1::tests::structured_owned_fixture::{
    Payload, call_match_fixture, formal_match_fixture, indexed_match_fixture, match_fixture,
    nested_match_fixture, string_match_fixture, vec_match_fixture,
};
use zryna_ir::data_ownership_v1 as ir;

fn parameter(lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>) -> Vec<raw::ValueDefinition> {
    let mut values = Vec::new();
    lowerer.mixed_function = true;
    if let Some(signature) = lowerer.catalog.modules[lowerer.module]
        .iter()
        .flatten()
        .find(|signature| signature.name == lowerer.function.name.text)
    {
        for (source_index, parameter) in signature.parameter_order.iter().enumerate() {
            if let crate::data_ownership_v1::function_catalog::FunctionParameterOrder::Value(
                index,
            ) = parameter
            {
                let syntax = &lowerer.function.parameters[source_index];
                let ty = signature.parameters[*index as usize];
                let at = span(lowerer.input.sources(), syntax.span);
                let value = raw::ValueDefinition { id: raw::ValueId(*index), ty: ty.ir, span: at };
                lowerer.constructor_types.record_parameter(&value).expect("dense parameter type");
                let place = raw::PlaceId(
                    u32::try_from(lowerer.places.len()).expect("parameter place count"),
                );
                lowerer.places.push(raw::Place {
                    id: place,
                    ty: ty.ir,
                    span: at,
                    kind: raw::PlaceKind::Parameter(*index),
                });
                lowerer
                    .bindings
                    .insert(syntax.name.text.clone(), Binding { ty, place, mutable: false });
                if !ty.is_copy() {
                    lowerer.owners.register_parameter(place).expect("genuine owned parameter");
                }
                values.push(value);
                continue;
            }
            let crate::data_ownership_v1::function_catalog::FunctionParameterOrder::Borrow(index) =
                parameter
            else {
                continue;
            };
            let descriptor = signature.borrow_parameters[*index as usize];
            let borrow = raw::BorrowId(*index);
            lowerer.preparation_facts.parameter_borrows.insert(borrow);
            lowerer.preparation_facts.aliases.insert(
                lowerer.function.parameters[source_index].name.text.clone(),
                super::super::preparation_plan::LexicalAlias {
                    borrow,
                    ty: descriptor.referent,
                    access: descriptor.access,
                },
            );
            lowerer.preparation_facts.next_borrow = borrow.0 + 1;
        }
    }
    lowerer.next_value = u32::try_from(values.len()).expect("parameter value count");
    values
}

#[test]
fn structured_cfg_resources_exact_extra_overflow_preserve_state_and_recover() {
    for shape in 0..8 {
        for resource in 0..5 {
            for extra in [0, 1, usize::MAX] {
                let (source, snapshot) = match shape {
                    0 => match_fixture(Payload::Struct, true, true),
                    1 => nested_match_fixture(true, true),
                    2 => call_match_fixture(true, true),
                    3 => vec_match_fixture(true, true),
                    4 => formal_match_fixture(true),
                    5 => string_match_fixture(0),
                    6 => string_match_fixture(1),
                    _ => indexed_match_fixture(false, true),
                };
                let errors = with_snapshot(&source, snapshot, |lowerer, result| {
                    let parameter = parameter(lowerer);
                    let initial = StructuredCheckpoint::capture(lowerer);
                    let pristine = lowerer
                        .lower_structured_cfg(&parameter, result)
                        .expect("pristine authenticated match");
                    let used = [
                        lowerer.next_value as usize,
                        lowerer.places.len(),
                        lowerer.instructions.len() + pristine.len(),
                        lowerer.cleanup_actions,
                        lowerer.cleanup_plans.len(),
                    ];
                    initial.restore(lowerer);
                    let maximum = [
                        ir::MAX_VALUES_PER_FUNCTION,
                        ir::MAX_PLACES_PER_FUNCTION,
                        ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION,
                        ir::MAX_DROP_ACTIONS_PER_FUNCTION,
                        ir::MAX_CLEANUP_PLANS_PER_FUNCTION,
                    ][resource];
                    let held =
                        if extra == usize::MAX { extra } else { maximum - used[resource] + extra };
                    match resource {
                        0 => lowerer.set_reserved_constructor_values_for_test(held),
                        1 => lowerer.set_reserved_constructor_places_for_test(held),
                        2 => lowerer.reserved_transitions = held,
                        3 => lowerer.preparation_facts.held_cleanup[1] = held,
                        4 => lowerer.preparation_facts.held_cleanup[0] = held,
                        _ => unreachable!("five resources"),
                    }
                    let before = format!(
                        "{:?}",
                        (
                            lowerer.preparation_checkpoint(),
                            &lowerer.bindings,
                            &lowerer.places,
                            &lowerer.instructions,
                            &lowerer.constructor_types,
                            &lowerer.owners,
                            &lowerer.preparation_facts
                        )
                    );
                    let output = lowerer.lower_structured_cfg(&parameter, result);
                    if extra == 0 {
                        assert_eq!(output, Some(pristine));
                    } else {
                        assert!(output.is_none(), "resource {resource}, extra {extra}");
                        assert_eq!(
                            format!(
                                "{:?}",
                                (
                                    lowerer.preparation_checkpoint(),
                                    &lowerer.bindings,
                                    &lowerer.places,
                                    &lowerer.instructions,
                                    &lowerer.constructor_types,
                                    &lowerer.owners,
                                    &lowerer.preparation_facts
                                )
                            ),
                            before
                        );
                        lowerer.set_reserved_constructor_values_for_test(0);
                        lowerer.set_reserved_constructor_places_for_test(0);
                        lowerer.reserved_transitions = 0;
                        lowerer.preparation_facts.held_cleanup = [0, 0];
                        assert_eq!(
                            lowerer.lower_structured_cfg(&parameter, result),
                            Some(pristine),
                            "pristine same-state recovery"
                        );
                    }
                });
                if extra == 0 {
                    assert!(errors.is_empty());
                } else {
                    assert_eq!(errors.len(), 1);
                    assert_eq!(errors[0].code(), "ZRYNA-M3201");
                }
            }
        }
    }
}

#[test]
fn structured_cfg_scratch_shares_authenticated_authority_and_stages_diagnostics() {
    let (source, snapshot) = nested_match_fixture(true, true);
    let errors = with_snapshot(&source, snapshot, |lowerer, _| {
        parameter(lowerer);
        let mut staged = super::super::Errors::new(lowerer.input.sources());
        let mut scratch = lowerer.structured_scratch(&mut staged);
        assert!(std::ptr::eq(scratch.function, lowerer.function));
        assert!(std::ptr::eq(scratch.file, lowerer.file));
        assert!(std::ptr::eq(scratch.layouts, lowerer.layouts));
        assert!(std::ptr::eq(scratch.catalog, lowerer.catalog));
        assert_eq!(scratch.places, lowerer.places);
        assert_eq!(scratch.instructions, lowerer.instructions);
        assert_eq!(scratch.preparation_checkpoint(), lowerer.preparation_checkpoint());
        assert_eq!(scratch.places.len(), 1);
        assert!(scratch.instructions.is_empty());
        scratch.places.clear();
        scratch.errors.at(
            "ZRYNA-M3015",
            span(scratch.input.sources(), scratch.function.span),
            "speculative rejection",
            "discard the speculative state",
        );
        assert_eq!(lowerer.places.len(), 1);
        assert!(lowerer.errors.is_empty());
        assert_eq!(staged.len(), 1);
    });
    assert!(errors.is_empty());
}

#[test]
fn structured_indexed_handoff_rejects_wrong_ssa_type_without_changing_state() {
    let (source, snapshot) = indexed_match_fixture(false, true);
    let errors = with_snapshot(&source, snapshot, |lowerer, result| {
        let parameters = parameter(lowerer);
        let initial = StructuredCheckpoint::capture(lowerer);
        let pristine = lowerer
            .lower_structured_cfg(&parameters, result)
            .expect("authenticated indexed baseline");
        initial.restore(lowerer);
        let index = lowerer
            .function
            .body
            .expressions
            .iter()
            .position(|expression| {
                matches!(expression.kind, zryna_syntax::v4::RawExpressionKind::Match { .. })
            })
            .expect("Match index");
        let integer = lowerer
            .node_types
            .iter()
            .flatten()
            .find(|ty| ty.category == TypeCategory::I32)
            .copied()
            .expect("i32 type");
        lowerer
            .preparation_facts
            .structured_values
            .insert(u32::try_from(index).expect("expression id"), (raw::ValueId(0), integer));
        let RawStatementKind::LocalDeclaration { initializer, .. } =
            lowerer.function.body.statements[0].kind
        else {
            panic!("authenticated initializer")
        };
        let before = format!(
            "{:?}",
            (
                &lowerer.preparation_facts,
                lowerer.preparation_checkpoint(),
                &lowerer.instructions,
                &lowerer.places,
                &lowerer.owners
            )
        );
        for _ in 0..2 {
            assert!(
                super::super::constructor_preparation::PreparedValue::prepare(
                    lowerer,
                    initializer,
                    result
                )
                .is_none()
            );
            assert_eq!(
                format!(
                    "{:?}",
                    (
                        &lowerer.preparation_facts,
                        lowerer.preparation_checkpoint(),
                        &lowerer.instructions,
                        &lowerer.places,
                        &lowerer.owners
                    )
                ),
                before
            );
        }
        lowerer.preparation_facts.structured_values.clear();
        assert_eq!(lowerer.lower_structured_cfg(&parameters, result), Some(pristine));
    });
    assert_eq!(errors.len(), 2);
    assert_eq!(errors[0], errors[1]);
    assert_eq!(errors[0].code(), "ZRYNA-M3016");
}
