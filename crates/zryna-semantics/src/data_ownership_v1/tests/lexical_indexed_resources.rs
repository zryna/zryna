use super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::constructor_resources::tests::{run_statement, with_snapshot};
use super::*;
use crate::data_ownership_v1::layout_graph::semantic_type;
use crate::data_ownership_v1::tests::explicit_indexed_fixture::{Action, Container, fixture};
use crate::data_ownership_v1::tests::generic_vec_fixture::Element;
use crate::data_ownership_v1::type_model::Binding;
use zryna_ir::data_ownership_v1 as ir;

pub(super) fn parameters(lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>) {
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
        .expect("parameter");
        let at = crate::data_ownership_v1::span(lowerer.input.sources(), parameter.span);
        let id = u32::try_from(index).expect("parameter index");
        lowerer
            .constructor_types
            .record_parameter(&raw::ValueDefinition { id: raw::ValueId(id), ty: ty.ir, span: at })
            .expect("dense parameter");
        let place = raw::PlaceId(id);
        lowerer.places.push(raw::Place {
            id: place,
            ty: ty.ir,
            span: at,
            kind: raw::PlaceKind::Parameter(id),
        });
        lowerer.bindings.insert(parameter.name.text.clone(), Binding { ty, place, mutable: false });
        if !ty.is_copy() {
            lowerer.owners.register_parameter(place).expect("owner");
        }
        lowerer.next_value += 1;
    }
}

pub(super) fn nested_statement(
    lowerer: &PrivateOwnedAggregateLowerer<'_, '_, '_>,
    ordinal: usize,
) -> u32 {
    lowerer.function.body.blocks[1].statements[ordinal]
}

#[test]
fn lexical_indexed_begin_credit_exact_extra_preserves_state_before_consumption() {
    for container in [Container::Vec, Container::Array(2)] {
        for extra in [false, true, false] {
            let (source, snapshot) =
                fixture(container, &Element::Struct, false, Action::Clone, None);
            let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
                crate::data_ownership_v1::lower(lowerer.input)
                    .expect("independent full source control");
                parameters(lowerer);
                assert!(run_statement(lowerer, 0, ty));
                assert!(run_statement(lowerer, 1, ty));
                let initial =
                    ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - lowerer.instructions.len() - 3
                        + usize::from(extra);
                lowerer.reserved_transitions = initial;
                let before = state(lowerer);
                let facts = lowerer.preparation_facts.clone();
                let count = lowerer.instructions.len();
                let id = nested_statement(lowerer, 0);
                let statement = lowerer.function.body.statements[id as usize].clone();
                assert_eq!(lowerer.lower_lexical_declaration(&statement).is_some(), !extra);
                if extra {
                    assert_eq!(state(lowerer), before);
                    assert_eq!(lowerer.preparation_facts, facts);
                } else {
                    assert_eq!(lowerer.instructions.len() - count, 2, "index read then begin");
                    assert_eq!(lowerer.reserved_transitions, initial + 1, "end remains reserved");
                    assert_eq!(lowerer.preparation_facts.active_borrows.len(), 1);
                    assert_eq!(lowerer.preparation_facts.aliases.len(), 1);
                }
                lowerer.reserved_transitions = 0;
            });
            assert_eq!(errors.len(), usize::from(extra));
            if extra {
                assert_eq!(errors[0].code(), "ZRYNA-M3201");
            }
        }
    }
}

#[test]
fn lexical_indexed_owned_local_drop_credit_is_part_of_scope_capacity() {
    for extra in [false, true, false] {
        let (source, snapshot) =
            fixture(Container::Vec, &Element::Struct, false, Action::Clone, None);
        let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
            crate::data_ownership_v1::lower(lowerer.input).expect("authenticated scope control");
            parameters(lowerer);
            assert!(run_statement(lowerer, 0, ty));
            assert!(run_statement(lowerer, 1, ty));
            let initial =
                ir::MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION - lowerer.instructions.len() - 6
                    + usize::from(extra);
            lowerer.reserved_transitions = initial;
            let pending = lowerer.owners.pending().to_vec();
            let count = lowerer.instructions.len();
            assert_eq!(run_statement(lowerer, 2, ty), !extra);
            assert_eq!(lowerer.reserved_transitions, initial, "scope restores incoming credits");
            assert!(!lowerer.bindings.contains_key("seen"));
            assert_eq!(lowerer.owners.pending(), pending.as_slice());
            if extra {
                assert_eq!(
                    lowerer.instructions.len() - count,
                    2,
                    "earlier begin may commit; failing local must not"
                );
                assert_eq!(lowerer.preparation_facts.active_borrows.len(), 1);
            } else {
                assert_eq!(lowerer.instructions.len() - count, 6);
                assert!(lowerer.preparation_facts.active_borrows.is_empty());
                assert!(lowerer.preparation_facts.aliases.is_empty());
                assert!(matches!(
                    lowerer.instructions.last().expect("scope drop").kind,
                    raw::InstructionKind::DropPlace { .. }
                ));
            }
            lowerer.reserved_transitions = 0;
        });
        assert_eq!(errors.len(), usize::from(extra));
        if extra {
            assert_eq!(errors[0].code(), "ZRYNA-M3201");
        }
    }
}

#[test]
fn lexical_indexed_borrowed_root_replacement_rejects_without_rhs_consumption() {
    for _ in 0..2 {
        let (source, snapshot) =
            fixture(Container::Vec, &Element::Struct, false, Action::OwnerReplace, None);
        let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
            parameters(lowerer);
            assert!(run_statement(lowerer, 0, ty));
            assert!(run_statement(lowerer, 1, ty));
            let declaration =
                lowerer.function.body.statements[nested_statement(lowerer, 0) as usize].clone();
            lowerer.lower_lexical_declaration(&declaration).expect("shared begin");
            let before = state(lowerer);
            let facts = lowerer.preparation_facts.clone();
            let id = nested_statement(lowerer, 1);
            let statement = lowerer.function.body.statements[id as usize].clone();
            assert!(lowerer.lower_statement(id, &statement, ty, None, 0).is_none());
            assert_eq!(state(lowerer), before);
            assert_eq!(lowerer.preparation_facts, facts);
            lowerer.reserved_transitions = 0;
        });
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code(), "ZRYNA-M3014");
    }
}
