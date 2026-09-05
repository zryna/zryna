use super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::constructor_resources::tests::{run_statement, with_snapshot};
use super::*;
use crate::data_ownership_v1::layout_graph::semantic_type;
use crate::data_ownership_v1::tests::generic_vec_fixture::{Element, Operation, fixture};
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
        let value = raw::ValueDefinition {
            id: raw::ValueId(u32::try_from(index).expect("parameter")),
            ty: ty.ir,
            span: at,
        };
        lowerer.constructor_types.record_parameter(&value).expect("dense parameters");
        let place = raw::PlaceId(u32::try_from(index).expect("place"));
        lowerer.places.push(raw::Place {
            id: place,
            ty: ty.ir,
            span: at,
            kind: raw::PlaceKind::Parameter(place.0),
        });
        lowerer.bindings.insert(parameter.name.text.clone(), Binding { ty, place, mutable: false });
        if !ty.is_copy() {
            lowerer.owners.register_parameter(place).expect("owned parameter");
        }
        lowerer.next_value += 1;
    }
}

#[test]
fn generic_vec_replacement_resource_exact_first_extra_reserves_bounds_rhs_commit_and_end() {
    for element in [Element::Struct, Element::Enum, Element::Array, Element::Vec] {
        for extra in [false, true, false] {
            let (source, snapshot) = fixture(&element, Operation::ReplaceClone, None);
            let mut expected = None;
            let errors = with_snapshot(&source, snapshot, |lowerer, ty| {
                parameters(lowerer);
                assert!(run_statement(lowerer, 0, ty));
                assert!(run_statement(lowerer, 1, ty));
                let pending = lowerer.owners.pending().to_vec();
                let demand = 3 * pending.len() + 1;
                let reserved = ir::MAX_DROP_ACTIONS_PER_FUNCTION - lowerer.cleanup_actions - demand
                    + usize::from(extra);
                lowerer.preparation_facts.held_cleanup[1] = reserved;
                let before = state(lowerer);
                let checkpoint = lowerer.preparation_checkpoint();
                let facts = lowerer.preparation_facts.clone();
                let plans = lowerer.cleanup_plans.len();
                let transitions = lowerer.instructions.len();
                let succeeded = run_statement(lowerer, 2, ty);
                assert_eq!(succeeded, !extra);
                if extra {
                    assert_eq!(state(lowerer), before);
                    assert_eq!(lowerer.preparation_checkpoint(), checkpoint);
                    assert_eq!(lowerer.preparation_facts, facts);
                    let RawStatementKind::Assignment { value, .. } =
                        lowerer.function.body.statements[2].kind
                    else {
                        panic!("assignment");
                    };
                    let at = lowerer.function.body.expressions[value as usize].span;
                    expected = Some(zryna_diagnostics::Diagnostic::error_at(
                        "ZRYNA-M3201",
                        crate::data_ownership_v1::span(lowerer.input.sources(), at),
                        "structural clone exceeds a checked value, place, or cleanup resource limit",
                        "reduce simultaneously live owned aggregates or clone sites",
                    ));
                } else {
                    assert_eq!(
                        lowerer.cleanup_actions + reserved,
                        ir::MAX_DROP_ACTIONS_PER_FUNCTION
                    );
                    assert_eq!(lowerer.cleanup_plans.len() - plans, 3);
                    assert_eq!(lowerer.instructions.len() - transitions, 5);
                    assert_eq!(lowerer.owners.pending(), pending.as_slice());
                    assert_eq!(lowerer.preparation_facts.held_cleanup[1], reserved);
                    assert!(matches!(
                        lowerer.instructions.last().expect("lexical end").kind,
                        raw::InstructionKind::EndBorrow { .. }
                    ));
                }
                lowerer.preparation_facts.held_cleanup[1] = 0;
                assert!(lowerer.constructor_storage_is_clear());
            });
            assert_eq!(errors, expected.into_iter().collect::<Vec<_>>());
        }
    }
}
