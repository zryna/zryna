use super::super::constructor_resources::tests::with_snapshot;
use super::super::structured_checkpoint::StructuredCheckpoint;
use super::*;
use crate::data_ownership_v1::Binding;
use crate::data_ownership_v1::tests::structured_owned_fixture::{
    Payload, call_match_fixture, continued_indexed_fixture, formal_match_fixture,
    fresh_indexed_literal_fixture, fresh_indexed_match_fixture, indexed_match_fixture,
    match_fixture, mixed_variant_fixture, nested_match_fixture, nested_variant_fixture,
    string_match_fixture, vec_match_fixture,
};
use zryna_ir::data_ownership_v1 as ir;

#[path = "structured_match_resources.rs"]
mod complete_enum;

pub(in crate::data_ownership_v1::owned_aggregate_lowering) fn parameter(
    lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>,
) -> Vec<raw::ValueDefinition> {
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

fn structured_resource_fixture(
    shape: usize,
) -> (String, zryna_syntax::v4::RawProjectSyntaxSnapshot) {
    match shape {
        0 => match_fixture(Payload::Struct, true, true),
        1 => nested_match_fixture(true, true),
        2 => call_match_fixture(true, true),
        3 => vec_match_fixture(true, true),
        4 => formal_match_fixture(true),
        5 => string_match_fixture(0),
        6 => string_match_fixture(1),
        7 => indexed_match_fixture(false, true),
        8..=15 => continued_indexed_fixture(shape & 1 != 0, shape & 2 != 0, shape & 4 != 0),
        16..=19 => {
            let fresh = shape - 16;
            fresh_indexed_match_fixture(fresh & 1 != 0, fresh & 2 != 0)
        }
        20..=25 => fresh_indexed_literal_fixture(shape & 1 != 0, (shape - 20) / 2),
        26 => mixed_variant_fixture(),
        27 => nested_variant_fixture(),
        _ => unreachable!("twenty-eight structured resource shapes"),
    }
}

fn exercise_structured_resource(
    lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>,
    result: Ty,
    resource: usize,
    extra: usize,
) {
    let parameter = parameter(lowerer);
    let initial = StructuredCheckpoint::capture(lowerer);
    let pristine =
        lowerer.lower_structured_cfg(&parameter, result).expect("pristine authenticated match");
    let used = [
        lowerer.next_value as usize,
        lowerer.places.len(),
        lowerer.instructions.len() + pristine.len(),
        lowerer.cleanup_actions,
        lowerer.cleanup_plans.len(),
    ];
    initial.restore(lowerer);
    let maximum = resource_limit(resource);
    let held = if extra == usize::MAX { extra } else { maximum - used[resource] + extra };
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
        return;
    }
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

fn assert_structured_resource_case(shape: usize, resource: usize, extra: usize) {
    let (source, snapshot) = structured_resource_fixture(shape);
    let errors = with_snapshot(&source, snapshot, |lowerer, result| {
        exercise_structured_resource(lowerer, result, resource, extra);
    });
    if extra == 0 {
        assert!(errors.is_empty());
        return;
    }
    assert_eq!(errors.len(), 1, "shape {shape}, resource {resource}, extra {extra}");
    assert_eq!(errors[0].code(), "ZRYNA-M3201");
    assert_call_overflow(shape, resource, extra, &source, &errors);
}

#[test]
fn structured_cfg_resources_exact_extra_overflow_preserve_state_and_recover() {
    for shape in 0..26 {
        for resource in 0..5 {
            for extra in [0, 1, usize::MAX] {
                assert_structured_resource_case(shape, resource, extra);
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

#[test]
fn structured_indexed_handoff_rejects_absent_stale_and_hostile_tickets_atomically() {
    for hostile in 0..4 {
        let (source, snapshot) = indexed_match_fixture(false, true);
        let errors = with_snapshot(&source, snapshot, |lowerer, result| {
            let _parameters = parameter(lowerer);
            let integer = lowerer
                .node_types
                .iter()
                .flatten()
                .find(|ty| ty.category == TypeCategory::I32)
                .copied()
                .expect("i32 type");
            let at = span(lowerer.input.sources(), lowerer.function.span);
            let valid = lowerer
                .emit(integer, at, raw::InstructionKind::I32Literal(0))
                .expect("genuine existing Copy value");
            let index = lowerer
                .function
                .body
                .expressions
                .iter()
                .position(|expression| {
                    matches!(expression.kind, zryna_syntax::v4::RawExpressionKind::Match { .. })
                })
                .and_then(|index| u32::try_from(index).ok())
                .expect("Match index");
            let RawStatementKind::LocalDeclaration { initializer, .. } =
                lowerer.function.body.statements[0].kind
            else {
                panic!("authenticated initializer")
            };
            match hostile {
                0 => {}
                1 => {
                    lowerer.preparation_facts.structured_values.insert(index + 1, (valid, integer));
                }
                2 => {
                    lowerer
                        .preparation_facts
                        .structured_values
                        .insert(index, (raw::ValueId(0), integer));
                }
                _ => {
                    lowerer.preparation_facts.structured_values.insert(index, (valid, result));
                }
            }
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
            assert!(
                super::super::constructor_preparation::PreparedValue::prepare(
                    lowerer,
                    initializer,
                    result
                )
                .is_none(),
                "hostile ticket {hostile}"
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
                before,
                "hostile ticket {hostile}"
            );
            lowerer.reserved_transitions = 0;
        });
        assert_eq!(errors.len(), 1, "hostile ticket {hostile}");
    }

    let (source, snapshot) = indexed_match_fixture(false, true);
    let errors = with_snapshot(&source, snapshot, |lowerer, result| {
        let parameters = parameter(lowerer);
        lowerer
            .lower_structured_cfg(&parameters, result)
            .expect("pristine lowering after hostile tickets");
    });
    assert!(errors.is_empty());
}

#[test]
fn structured_indexed_handoff_ticket_is_consumed_once() {
    let (source, snapshot) = indexed_match_fixture(false, true);
    let errors = with_snapshot(&source, snapshot, |lowerer, result| {
        let _parameters = parameter(lowerer);
        let integer = lowerer
            .node_types
            .iter()
            .flatten()
            .find(|ty| ty.category == TypeCategory::I32)
            .copied()
            .expect("i32 type");
        let at = span(lowerer.input.sources(), lowerer.function.span);
        let value = lowerer
            .emit(integer, at, raw::InstructionKind::I32Literal(0))
            .expect("genuine existing Copy value");
        let index = lowerer
            .function
            .body
            .expressions
            .iter()
            .position(|expression| {
                matches!(expression.kind, zryna_syntax::v4::RawExpressionKind::Match { .. })
            })
            .and_then(|index| u32::try_from(index).ok())
            .expect("Match index");
        let RawStatementKind::LocalDeclaration { initializer, .. } =
            lowerer.function.body.statements[0].kind
        else {
            panic!("authenticated initializer")
        };
        lowerer.preparation_facts.structured_values.insert(index, (value, integer));
        super::super::constructor_preparation::PreparedValue::prepare(lowerer, initializer, result)
            .expect("first ticket use")
            .consume();
        assert!(!lowerer.preparation_facts.structured_values.contains_key(&index));
        let before = lowerer.preparation_checkpoint();
        assert!(
            super::super::constructor_preparation::PreparedValue::prepare(
                lowerer,
                initializer,
                result
            )
            .is_none(),
            "consumed ticket cannot be replayed"
        );
        assert_eq!(lowerer.preparation_checkpoint(), before);
        lowerer.reserved_transitions = 0;
    });
    assert_eq!(errors.len(), 1);
}

fn assert_call_overflow(
    shape: usize,
    resource: usize,
    extra: usize,
    source: &str,
    errors: &[zryna_diagnostics::Diagnostic],
) {
    if (8..12).contains(&shape) && extra == usize::MAX && matches!(resource, 0 | 4) {
        assert_eq!(
            errors[0].message(),
            if resource == 0 {
                "call result resource reservation overflowed"
            } else {
                "call cleanup resource reservation overflowed"
            }
        );
        assert_eq!(
            errors[0].guidance(),
            if resource == 0 {
                "reduce simultaneously reserved values, owners, or transitions"
            } else {
                "reduce simultaneously reserved cleanup plans or actions"
            }
        );
        let at = errors[0].primary_span().expect("source call span");
        let text = "indexValue(offset, \"index\")";
        let offset = source.find(text).expect("effectful index call");
        assert_eq!((at.start() as usize, at.end() as usize), (offset, offset + text.len()));
    }
}

fn resource_limit(resource: usize) -> usize {
    [
        ir::MAX_VALUES_PER_FUNCTION,
        ir::MAX_PLACES_PER_FUNCTION,
        ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION,
        ir::MAX_DROP_ACTIONS_PER_FUNCTION,
        ir::MAX_CLEANUP_PLANS_PER_FUNCTION,
    ][resource]
}
