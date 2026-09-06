use super::super::constructor_resources::tests::with_snapshot;
use super::super::structured_checkpoint::StructuredCheckpoint;
use super::*;
use crate::data_ownership_v1::layout_graph::semantic_type;
use crate::data_ownership_v1::tests::generic_vec_fixture::weak_upgrade_fixture::fixture;
use crate::data_ownership_v1::{Binding, span};
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
    .expect("exact payload parameter");
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
    lowerer.owners.register_parameter(raw::PlaceId(0)).expect("owned parameter");
    lowerer.next_value = 1;
    lowerer.mixed_function = true;
    value
}

#[test]
fn weak_upgrade_exact_and_first_extra_resources_restore_pristine_state() {
    for temporary in [false, true] {
        for resource in 0..5 {
            for extra in 0..=1 {
                let (source, snapshot) = fixture(temporary);
                let errors = with_snapshot(&source, snapshot, |lowerer, result| {
                    let parameter = parameter(lowerer);
                    let initial = StructuredCheckpoint::capture(lowerer);
                    let pristine = lowerer
                        .lower_structured_cfg(&[parameter], result)
                        .expect("pristine authenticated upgrade");
                    let used = [
                        lowerer.next_value as usize,
                        lowerer.places.len(),
                        lowerer.instructions.len() + pristine.len() + 2,
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
                    let held = maximum - used[resource] + extra;
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
                    let output = lowerer.lower_structured_cfg(&[parameter], result);
                    if extra == 0 {
                        assert_eq!(
                            output,
                            Some(pristine),
                            "resource {resource}, temporary {temporary}"
                        );
                    } else {
                        assert!(output.is_none(), "resource {resource}, temporary {temporary}");
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
                            lowerer.lower_structured_cfg(&[parameter], result),
                            Some(pristine),
                            "pristine recovery"
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
