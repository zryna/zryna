use super::*;
use crate::data_ownership_v1::owned_aggregate_lowering::availability::AvailabilityView;
use crate::data_ownership_v1::owned_aggregate_lowering::operand_decisions::ReferenceKind;
use crate::data_ownership_v1::type_model::Binding;
use zryna_ir::data_ownership_v1::raw;
use zryna_layout::TypeCategory;
use zryna_source::Span;
use zryna_syntax::v4::RawIdentifierSyntax;

#[path = "operand_clone_tests.rs"]
mod clones;
#[path = "operand_projection_tests.rs"]
mod projections;

#[derive(Debug, PartialEq, Eq)]
struct State {
    instructions: Vec<raw::Instruction>,
    places: Vec<raw::Place>,
    cleanup: Vec<raw::CleanupPlan>,
    owners: OwnerState,
    bindings: BTreeMap<String, Binding>,
    projections: BTreeMap<(u32, u8, u32), raw::PlaceId>,
    moved: BTreeSet<raw::PlaceId>,
    partial: BTreeSet<raw::PlaceId>,
    counts: [usize; 8],
    credits: [usize; 4],
}

fn state(lowerer: &PrivateOwnedAggregateLowerer<'_, '_, '_>) -> State {
    State {
        instructions: lowerer.instructions.clone(),
        places: lowerer.places.clone(),
        cleanup: lowerer.cleanup_plans.clone(),
        owners: lowerer.owners.clone(),
        bindings: lowerer.bindings.clone(),
        projections: lowerer.projections.clone(),
        moved: lowerer.moved_projections.clone(),
        partial: lowerer.partial_roots.clone(),
        counts: [
            lowerer.next_value as usize,
            lowerer.next_local as usize,
            lowerer.cleanup_actions,
            lowerer.aggregate_operands,
            lowerer.aggregate_subobject_moves,
            lowerer.projected_aggregate_clones,
            lowerer.projected_aggregate_assignments,
            lowerer.reserved_transitions,
        ],
        credits: credits(lowerer),
    }
}

fn ty(lowerer: &PrivateOwnedAggregateLowerer<'_, '_, '_>, category: TypeCategory) -> Ty {
    lowerer
        .node_types
        .iter()
        .flatten()
        .find(|ty| ty.category == category)
        .copied()
        .expect("fixture type")
}

fn expression_span(lowerer: &PrivateOwnedAggregateLowerer<'_, '_, '_>, id: u32) -> Span {
    span(lowerer.input.sources(), lowerer.expression(id).expect("fixture expression").span)
}

fn reference(lowerer: &PrivateOwnedAggregateLowerer<'_, '_, '_>, id: u32) -> RawIdentifierSyntax {
    let RawExpressionKind::Reference { name } =
        &lowerer.expression(id).expect("fixture reference").kind
    else {
        panic!("reference")
    };
    name.clone()
}

fn assert_error(errors: &[Diagnostic], code: &str, message: &str, help: &str, at: Span) {
    assert_eq!(errors, &[Diagnostic::error_at(code, at, message, help)]);
}

#[test]
fn operand_decisions_reference_lookup_type_and_owner_order() {
    for name in ["P", "q"] {
        let (mut source, mut snapshot) = fixtures::snapshot(Fixture::Pair);
        let expression =
            snapshot.files[0].functions[0].body.expressions.last_mut().expect("return");
        let at = expression.span;
        source.replace_range(at.start as usize..at.end as usize, name);
        expression.kind = RawExpressionKind::Reference {
            name: RawIdentifierSyntax { text: name.to_owned(), span: at },
        };
        let mut expected_span = None;
        let errors = with_snapshot(&source, snapshot, |lowerer, result| {
            assert!(run_statement(lowerer, 0, result));
            let id = root_value(lowerer, 1);
            expected_span = Some(expression_span(lowerer, id));
            let before = state(lowerer);
            assert!(
                lowerer
                    .reference_value(&reference(lowerer, id), result, expected_span.expect("span"))
                    .is_none()
            );
            assert_eq!(state(lowerer), before);
        });
        let message = if name == "P" {
            "aggregate value 'P' has the wrong portable ASCII case"
        } else {
            "aggregate value 'q' is not declared"
        };
        assert_error(
            &errors,
            "ZRYNA-M3002",
            message,
            "reference one exact preceding local using its declared spelling",
            expected_span.expect("span"),
        );
    }
    for wrong_type in [true, false] {
        let mut at = None;
        let errors = with_fixture(Fixture::Pair, |lowerer, result| {
            assert!(run_statement(lowerer, 0, result));
            let name = reference(lowerer, root_value(lowerer, 1));
            at = Some(span(lowerer.input.sources(), name.span));
            let root = lowerer.bindings[&name.text].place;
            lowerer.partial_roots.insert(root); // Deliberate availability-state negative case.
            let expected = if wrong_type { ty(lowerer, TypeCategory::Bool) } else { result };
            let before = state(lowerer);
            assert!(lowerer.operand_decisions().reference_decision(&name, expected).is_none());
            assert_eq!(state(lowerer), before);
        });
        assert_error(
            &errors,
            if wrong_type { "ZRYNA-M3016" } else { "ZRYNA-M3014" },
            if wrong_type {
                "aggregate operand has the wrong exact type"
            } else {
                "aggregate value 'p' is moved or only partially available"
            },
            if wrong_type {
                "use the exact declared field, element, local, or result type"
            } else {
                "move a whole owned aggregate only before moving any of its projections"
            },
            at.expect("span"),
        );
    }
}

#[test]
fn operand_decisions_copy_binding_does_not_require_pending_cleanup_ownership() {
    let errors = with_fixture(Fixture::Pair, |lowerer, result| {
        assert!(run_statement(lowerer, 0, result));
        let name = reference(lowerer, root_value(lowerer, 1));
        let copy = ty(lowerer, TypeCategory::Bool);
        let place = lowerer.bindings[&name.text].place;
        // Binding/type view only: not a claim this mutated fixture is valid complete IR.
        lowerer.bindings.get_mut(&name.text).expect("binding").ty = copy;
        lowerer.owners.consume_owner(place).expect("pending owner");
        lowerer.partial_roots.insert(place);
        let before = state(lowerer);
        let decision =
            lowerer.operand_decisions().reference_decision(&name, copy).expect("Copy bypass");
        assert!(matches!(decision.kind, ReferenceKind::Copy));
        assert_eq!(decision.binding.place, place);
        assert_eq!(state(lowerer), before);
    });
    assert!(errors.is_empty());
}

#[test]
fn operand_decisions_whole_move_rehomes_the_original_pending_slot() {
    let errors = with_fixture(Fixture::WholeClone, |lowerer, result| {
        assert!(run_statement(lowerer, 0, result));
        let clone = root_value(lowerer, 1);
        let RawExpressionKind::Clone { value: operand, .. } =
            lowerer.expression(clone).expect("clone").kind
        else {
            panic!("clone")
        };
        let name = reference(lowerer, operand);
        let at = expression_span(lowerer, operand);
        lowerer.value(clone, result).expect("distinct clone owner");
        let pending = lowerer.owners.pending().to_vec();
        assert_eq!(pending.len(), 2);
        let value = lowerer.reference_value(&name, result, at).expect("whole move");
        let moved = lowerer.owners.owner(value).expect("move owner");
        assert_eq!(lowerer.owners.pending(), &[moved, pending[1]]);
        assert!(!lowerer.whole_root_available(pending[0]));
        assert!(matches!(lowerer.instructions.last().expect("move").kind,
            raw::InstructionKind::MoveFromPlace { place } if place == pending[0]));
    });
    assert!(errors.is_empty());
}

#[test]
fn operand_decisions_availability_preserves_ancestry_overlap_and_cycle_termination() {
    let mut owners = OwnerState::default();
    owners.register_parameter(raw::PlaceId(0)).expect("root");
    // A topology-only view: 0 -> 1 -> 2 and disjoint sibling 3; not raw IR.
    let parent = |place: raw::PlaceId| match place.0 {
        1 | 3 => Some(raw::PlaceId(0)),
        2 => Some(raw::PlaceId(1)),
        _ => None,
    };
    for moved in [0, 1, 2, 3] {
        let masks = BTreeSet::from([raw::PlaceId(moved)]);
        let partial = BTreeSet::from([raw::PlaceId(0)]);
        let view = AvailabilityView::new(&owners, &masks, &partial, parent);
        assert_eq!(view.projection_available(raw::PlaceId(2), raw::PlaceId(0)), moved == 3);
        assert!(view.places_overlap(raw::PlaceId(1), raw::PlaceId(2)));
        assert!(view.places_overlap(raw::PlaceId(2), raw::PlaceId(1)));
        assert!(!view.whole_root_available(raw::PlaceId(0)));
    }
    let masks = BTreeSet::new();
    let partial = BTreeSet::new();
    let cyclic = AvailabilityView::new(&owners, &masks, &partial, |place: raw::PlaceId| {
        Some(raw::PlaceId(1 - place.0))
    });
    assert!(!cyclic.place_is_at_or_below(raw::PlaceId(0), raw::PlaceId(7)));
}
