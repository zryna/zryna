use super::super::BorrowIdentity;
use super::indexed_borrow_fixture::{Container, Element, Fixture};
use super::transient_indexed_edges::diamond;
use super::*;

#[test]
fn transient_indexed_upgrade_preserves_both_outcomes_and_unwinds_count_failure() {
    let fixture = Fixture::new(Container::Array, Element::Weak);
    let shared = fixture
        .linear
        .types()
        .find(|ty| ty.category() == zryna_layout::TypeCategory::Shared)
        .unwrap()
        .id()
        .index();
    let mut program = diamond(&fixture, raw::BorrowAccess::Shared);
    let function = &mut program.modules[0].functions[0];
    function.blocks.pop();
    function.cleanup_plans[2].actions.insert(0, raw::DropAction::DropPlace(raw::PlaceId(3)));
    let mut count_cleanup = function.cleanup_plans[0].clone();
    count_cleanup.id = raw::CleanupPlanId(3);
    function.cleanup_plans.push(count_cleanup);
    function.blocks[0].terminators[0].kind = raw::Terminator::WeakUpgradeBranch {
        weak: raw::PlaceId(2),
        success: raw::Edge { target: raw::BlockId(1), arguments: vec![] },
        expired: raw::Edge { target: raw::BlockId(2), arguments: vec![] },
        cleanup: raw::CleanupPlanId(3),
    };
    function.blocks[1].parameters = vec![raw::ValueDefinition {
        id: raw::ValueId(6),
        ty: raw::TypeId(shared),
        span: function.span,
    }];
    function.places.push(raw::Place {
        id: raw::PlaceId(3),
        ty: raw::TypeId(shared),
        span: function.span,
        kind: raw::PlaceKind::Temporary(raw::ValueId(6)),
    });
    for (block, cleanup) in [(1, 2), (2, 1)] {
        function.blocks[block].instructions = vec![end_borrow(0, function.span)];
        function.blocks[block].terminators[0].kind = raw::Terminator::Return {
            value: raw::ValueId(2),
            cleanup: raw::CleanupPlanId(cleanup),
        };
    }
    let verified = fixture.verify(program.clone());
    let function = verified.modules().next().unwrap().functions().next().unwrap();
    let branch = function.blocks().next().unwrap().terminator();
    assert_eq!(branch.continued_indexed_accesses().next().unwrap().borrow().index(), 0);
    assert_eq!(branch.failure_ended_borrows().map(BorrowIdentity::index).collect::<Vec<_>>(), [0]);
    assert_eq!(branch.derived_drop_actions().count(), 3);
    assert_eq!(function.blocks().nth(1).unwrap().terminator().derived_drop_actions().count(), 4);
    assert_eq!(function.blocks().nth(2).unwrap().terminator().derived_drop_actions().count(), 3);
    for block in [1, 2] {
        let mut hostile = program.clone();
        hostile.modules[0].functions[0].blocks[block].instructions.clear();
        fixture.rejects(hostile, "ZRYNA-I3011");
    }
    fixture.verify(program);
}
