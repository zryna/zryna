use super::indexed_borrow_fixture::{Container, Element, Fixture};
use super::*;
use crate::data_ownership_v1::{BorrowIdentity, VerifiedGenericCloneSource};

pub(super) fn clone_seed(fixture: &Fixture, access: raw::BorrowAccess) -> raw::Program {
    let mut raw = fixture.seed(access);
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    function.result = fixture.element;
    function.places.push(raw::Place {
        id: raw::PlaceId(3),
        ty: fixture.element,
        span,
        kind: raw::PlaceKind::Temporary(raw::ValueId(5)),
    });
    function.blocks[0].instructions.insert(
        1,
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(5), ty: fixture.element, span }),
            span,
            kind: raw::InstructionKind::GenericCloneBorrow {
                borrow: raw::BorrowId(0),
                cleanup: raw::CleanupPlanId(1),
                prefix_cleanup: raw::CleanupPlanId(2),
            },
        },
    );
    let pending = function.cleanup_plans[0].actions.clone();
    let mut prefix = vec![raw::DropAction::DropGenericCloneInitializedPrefix(raw::PlaceId(3))];
    prefix.extend(pending.clone());
    function.cleanup_plans.push(raw::CleanupPlan {
        id: raw::CleanupPlanId(2),
        span,
        actions: prefix,
    });
    function.cleanup_plans.push(raw::CleanupPlan {
        id: raw::CleanupPlanId(3),
        span,
        actions: pending,
    });
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(5), cleanup: raw::CleanupPlanId(3) };
    raw
}

#[test]
fn generic_vec_observation_clones_exact_owned_referent_without_container_move_or_holes() {
    for container in [Container::Array, Container::Vec] {
        for element in
            [Element::String, Element::Struct, Element::Enum, Element::Array, Element::Vec]
        {
            let fixture = Fixture::new(container, element);
            for access in [raw::BorrowAccess::Shared, raw::BorrowAccess::Exclusive] {
                let seed = clone_seed(&fixture, access);
                let mut previous = None;
                for _ in 0..2 {
                    let verified = fixture.verify(seed.clone());
                    let function = verified
                        .modules()
                        .next()
                        .expect("module")
                        .functions()
                        .next()
                        .expect("function");
                    let block = function.blocks().next().expect("block");
                    let instructions = block.instructions().collect::<Vec<_>>();
                    let begin = instructions[0].indexed_borrow().expect("bounds authority");
                    let clone = instructions[1].generic_clone().expect("owned observation");
                    assert!(
                        matches!(clone.source(), VerifiedGenericCloneSource::Borrow(borrow) if borrow == begin.borrow())
                    );
                    assert_eq!(clone.ty(), begin.referent());
                    assert_eq!(clone.ty().index(), fixture.element.0);
                    assert_eq!(clone.destination().index(), 3);
                    assert_eq!(clone.result().index(), 5);
                    assert_eq!(
                        instructions[1]
                            .failure_ended_borrows()
                            .map(BorrowIdentity::index)
                            .collect::<Vec<_>>(),
                        [0]
                    );
                    let bounds = instructions[0].derived_drop_actions().collect::<Vec<_>>();
                    let prepare = instructions[1].derived_drop_actions().collect::<Vec<_>>();
                    let prefix = instructions[1]
                        .generic_clone_prefix_failure_drop_actions()
                        .collect::<Vec<_>>();
                    assert_eq!(bounds, prepare);
                    assert_eq!(&prefix[1..], prepare.as_slice());
                    assert_eq!(
                        prefix[0].kind(),
                        VerifiedDropActionKind::GenericCloneInitializedPrefix
                    );
                    assert_eq!(prefix[0].root(), clone.destination());
                    assert_eq!(
                        prepare,
                        block.terminator().derived_drop_actions().collect::<Vec<_>>()
                    );
                    assert!(prepare.iter().all(|drop| drop.moved_projections().count() == 0));
                    assert_eq!(instructions[2].borrow(), Some(begin.borrow()));
                    let trace = (prepare, prefix);
                    if let Some(previous) = previous.replace(trace.clone()) {
                        assert_eq!(previous, trace);
                    }
                }
            }
        }
    }
}

#[test]
fn generic_vec_observation_distinct_clone_owner_replaces_the_exclusively_borrowed_element() {
    for element in [Element::String, Element::Struct, Element::Enum, Element::Array, Element::Vec] {
        let fixture = Fixture::new(Container::Vec, element);
        let mut raw = clone_seed(&fixture, raw::BorrowAccess::Exclusive);
        let function = &mut raw.modules[0].functions[0];
        function.result = fixture.integer;
        function.blocks[0].instructions.insert(
            2,
            raw::Instruction {
                result: None,
                span: function.span,
                kind: raw::InstructionKind::BorrowReplace {
                    borrow: raw::BorrowId(0),
                    value: raw::ValueId(5),
                },
            },
        );
        function.blocks[0].terminators[0].kind =
            raw::Terminator::Return { value: raw::ValueId(2), cleanup: raw::CleanupPlanId(3) };
        let verified = fixture.verify(raw);
        let block = verified
            .modules()
            .next()
            .expect("module")
            .functions()
            .next()
            .expect("function")
            .blocks()
            .next()
            .expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        let clone = instructions[1].generic_clone().expect("prepared clone");
        let replace = instructions[2].borrow_replacement().expect("commit");
        assert_eq!(replace.value(), clone.result());
        assert_eq!(replace.referent(), clone.ty());
        assert_eq!(replace.old_value_drop().referent(), clone.ty());
        assert_eq!(instructions[2].derived_drop_actions().count(), 0);
        assert_eq!(
            instructions[1].derived_drop_actions().collect::<Vec<_>>(),
            block.terminator().derived_drop_actions().collect::<Vec<_>>()
        );
    }
}
