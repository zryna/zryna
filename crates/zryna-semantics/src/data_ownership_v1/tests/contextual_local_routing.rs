use super::super::constructor_resources::ConstructorStorage;
use super::super::constructor_resources::tests::child_preparation_red::state;
use super::super::constructor_resources::tests::{root_value, with_fixture, with_snapshot};
use super::super::mixed_shape::{PreparationRoute, route};
use super::super::preparation_plan::PreparationFacts;
use super::*;
use crate::data_ownership_v1::OwnerState;
use crate::data_ownership_v1::layout_graph::semantic_type;
use crate::data_ownership_v1::owned_constructor_plan::ConstructorValueTypes;
use crate::data_ownership_v1::span;
use crate::data_ownership_v1::tests::constructor_envelope_fixtures::Fixture;
use crate::data_ownership_v1::tests::mixed_vec_siblings::vec_sibling_fixture;
use std::collections::{BTreeMap, BTreeSet};
use zryna_diagnostics::Diagnostic;
use zryna_syntax::v4::RawStatementKind;

#[test]
fn contextual_vec_local_route_does_not_reclassify_root_topology() {
    let (source, snapshot) = vec_sibling_fixture(false);
    for local in [false, true] {
        let mut expected = None;
        let errors = with_snapshot(&source, snapshot.clone(), |lowerer, _| {
            assert!(lowerer.mixed_function, "actual authenticated mixed function context");
            let RawStatementKind::LocalDeclaration { type_syntax, initializer, .. } =
                lowerer.function.body.statements[0].kind
            else {
                panic!("real Vec producer local")
            };
            let ty = semantic_type(
                lowerer.file,
                type_syntax,
                lowerer.module,
                lowerer.declarations,
                lowerer.graph,
                lowerer.node_types,
                lowerer.errors,
            )
            .expect("authenticated Vec local type");
            assert_eq!(route(ty, lowerer.layouts), PreparationRoute::LegacyVec);
            assert_eq!(lowerer.local_preparation_route(ty), PreparationRoute::MixedSummary);
            let before = state(lowerer);
            let facts = lowerer.preparation_facts.clone();
            if local {
                let prepared = PreparedValue::prepare_local(lowerer, initializer, ty)
                    .expect("contextual local uses shared summary");
                assert_eq!(state(prepared.lowerer), before);
                assert_eq!(prepared.lowerer.preparation_facts, facts);
                let value = prepared.consume();
                assert_eq!(value, raw::ValueId(0));
                assert_eq!(lowerer.instructions.len(), 1);
                assert!(matches!(
                    lowerer.instructions[0].kind,
                    raw::InstructionKind::DirectCall { .. }
                ));
                assert_eq!(lowerer.owners.pending(), &[raw::PlaceId(0)]);
                assert!(lowerer.preparation_facts.string_bytes.is_empty());
            } else {
                expected = Some(Diagnostic::error_at(
                    "ZRYNA-M3016",
                    span(
                        lowerer.input.sources(),
                        lowerer.function.body.expressions[initializer as usize].span,
                    ),
                    "scalar and String Vec roots require their existing ordered lowering route",
                    "keep this Vec root on its established construction authority",
                ));
                assert!(PreparedValue::prepare(lowerer, initializer, ty).is_none());
                assert_eq!(state(lowerer), before);
                assert_eq!(lowerer.preparation_facts, facts);
            }
            assert_eq!(route(ty, lowerer.layouts), PreparationRoute::LegacyVec);
        });
        assert_eq!(errors, expected.into_iter().collect::<Vec<_>>());
    }
}

#[test]
fn contextual_local_entry_preserves_complete_aggregate_preparation_schedule() {
    let errors = with_fixture(Fixture::Pair, |lowerer, ty| {
        // Both lowerers borrow the SAME authenticated source/layout/catalog context.
        let mut peer_errors = crate::data_ownership_v1::Errors::new(lowerer.input.sources());
        let mut peer = PrivateOwnedAggregateLowerer {
            input: lowerer.input,
            file: lowerer.file,
            function: lowerer.function,
            module: lowerer.module,
            declarations: lowerer.declarations,
            graph: lowerer.graph,
            node_types: lowerer.node_types,
            layouts: lowerer.layouts,
            catalog: lowerer.catalog,
            mixed_function: lowerer.mixed_function,
            errors: &mut peer_errors,
            bindings: BTreeMap::default(),
            projections: BTreeMap::default(),
            moved_projections: BTreeSet::default(),
            partial_roots: BTreeSet::default(),
            places: Vec::new(),
            instructions: Vec::new(),
            constructor_types: ConstructorValueTypes::default(),
            constructor_storage: ConstructorStorage::default(),
            preparation_facts: PreparationFacts::default(),
            cleanup_plans: Vec::new(),
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
        assert_eq!(state(lowerer), state(&peer), "identical genuine initial states");
        assert_eq!(lowerer.preparation_facts, peer.preparation_facts);
        let original = ordinary_outcome(lowerer, ty, false);
        let local = ordinary_outcome(&mut peer, ty, true);
        assert_eq!(local, original, "local entry preserves complete schedule and state");
        assert!(peer_errors.is_empty());
    });
    assert!(errors.is_empty());
}

fn ordinary_outcome(
    lowerer: &mut PrivateOwnedAggregateLowerer<'_, '_, '_>,
    ty: Ty,
    local: bool,
) -> (
    super::super::constructor_resources::tests::child_preparation_red::PreparationState,
    PreparationFacts,
) {
    assert!(!lowerer.mixed_function, "ordinary complete aggregate context");
    assert_eq!(route(ty, lowerer.layouts), PreparationRoute::Aggregate);
    assert_eq!(lowerer.local_preparation_route(ty), PreparationRoute::Aggregate);
    let initializer = root_value(lowerer, 0);
    let before = state(lowerer);
    let prepared = if local {
        PreparedValue::prepare_local(lowerer, initializer, ty)
    } else {
        PreparedValue::prepare(lowerer, initializer, ty)
    }
    .expect("same complete aggregate initializer");
    assert_eq!(state(prepared.lowerer), before);
    let value = prepared.consume();
    assert_eq!(value, raw::ValueId(2));
    assert_eq!(lowerer.instructions.len(), 3);
    assert_eq!(lowerer.places.len(), 2);
    assert_eq!(lowerer.owners.pending(), &[raw::PlaceId(1)]);
    assert_eq!(lowerer.owners.owner(value), Some(raw::PlaceId(1)));
    (state(lowerer), lowerer.preparation_facts.clone())
}
