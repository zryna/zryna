use super::super::constructor_resources::tests::with_snapshot;
use super::super::structured_checkpoint::StructuredCheckpoint;
use super::*;
use crate::data_ownership_v1::Binding;
use crate::data_ownership_v1::tests::structured_owned_fixture::{
    Payload, match_fixture, nested_match_fixture,
};
use zryna_ir::data_ownership_v1 as ir;

fn parameter(lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>) -> raw::ValueDefinition {
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
    .expect("exact parameter type");
    let at = span(lowerer.input.sources(), parameter.span);
    let value = raw::ValueDefinition { id: raw::ValueId(0), ty: ty.ir, span: at };
    lowerer.constructor_types.record_parameter(&value).expect("dense parameter type");
    lowerer.places.push(raw::Place {
        id: raw::PlaceId(0),
        ty: ty.ir,
        span: at,
        kind: raw::PlaceKind::Parameter(0),
    });
    lowerer.bindings.insert(
        parameter.name.text.clone(),
        Binding { ty, place: raw::PlaceId(0), mutable: false },
    );
    lowerer.owners.register_parameter(raw::PlaceId(0)).expect("genuine parameter owner");
    lowerer.next_value = 1;
    lowerer.mixed_function = true;
    value
}

#[test]
fn structured_cfg_resources_exact_extra_overflow_preserve_state_and_recover() {
    for nested in [false, true] {
        for resource in 0..4 {
            for extra in [0, 1, usize::MAX] {
                let (source, snapshot) = if nested {
                    nested_match_fixture(true, true)
                } else {
                    match_fixture(Payload::Struct, true, true)
                };
                let errors = with_snapshot(&source, snapshot, |lowerer, result| {
                    let parameter = parameter(lowerer);
                    let initial = StructuredCheckpoint::capture(lowerer);
                    let pristine = lowerer
                        .lower_structured_cfg(&[parameter], result)
                        .expect("pristine authenticated match");
                    let used = [
                        lowerer.next_value as usize,
                        lowerer.places.len(),
                        lowerer.instructions.len() + pristine.len(),
                        lowerer.cleanup_actions,
                    ];
                    initial.restore(lowerer);
                    let maximum = [
                        ir::MAX_VALUES_PER_FUNCTION,
                        ir::MAX_PLACES_PER_FUNCTION,
                        ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION,
                        ir::MAX_DROP_ACTIONS_PER_FUNCTION,
                    ][resource];
                    let held =
                        if extra == usize::MAX { extra } else { maximum - used[resource] + extra };
                    match resource {
                        0 => lowerer.set_reserved_constructor_values_for_test(held),
                        1 => lowerer.set_reserved_constructor_places_for_test(held),
                        2 => lowerer.reserved_transitions = held,
                        3 => lowerer.preparation_facts.held_cleanup[1] = held,
                        _ => unreachable!("four resources"),
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
                    let output = lowerer.lower_structured_cfg(&[parameter], result);
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
                        lowerer.preparation_facts.held_cleanup[1] = 0;
                        assert_eq!(
                            lowerer.lower_structured_cfg(&[parameter], result),
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
