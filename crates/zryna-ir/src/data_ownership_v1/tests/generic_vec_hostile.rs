use super::generic_vec_observation::clone_seed;
use super::indexed_borrow_fixture::{Container, Element, Fixture};
use super::*;

#[test]
fn generic_vec_observation_rejects_inactive_foreign_and_wrong_referent_authority() {
    for element in [Element::String, Element::Struct, Element::Enum, Element::Array, Element::Vec] {
        let fixture = Fixture::new(Container::Vec, element);
        let seed = clone_seed(&fixture, raw::BorrowAccess::Shared);
        fixture.verify(seed.clone());
        let mut ended = seed.clone();
        ended.modules[0].functions[0].blocks[0].instructions.swap(1, 2);
        fixture.rejects(ended, "ZRYNA-I3011");
        let mut foreign = seed.clone();
        let raw::InstructionKind::GenericCloneBorrow { borrow, .. } =
            &mut foreign.modules[0].functions[0].blocks[0].instructions[1].kind
        else {
            panic!("clone");
        };
        *borrow = raw::BorrowId(99);
        fixture.rejects(foreign, "ZRYNA-I3005");
        let mut wrong = seed;
        let function = &mut wrong.modules[0].functions[0];
        function.result = fixture.integer;
        function.places[3].ty = fixture.integer;
        function.blocks[0].instructions[1].result.as_mut().expect("result").ty = fixture.integer;
        fixture.rejects(wrong, "ZRYNA-I3005");
    }
}

#[test]
fn generic_vec_observation_rejects_wrong_prefix_source_alias_and_premature_result_cleanup() {
    let fixture = Fixture::new(Container::Vec, Element::Enum);
    let seed = clone_seed(&fixture, raw::BorrowAccess::Shared);
    fixture.verify(seed.clone());
    for (mutation, code) in
        [(0, "ZRYNA-I3013"), (1, "ZRYNA-I3012"), (2, "ZRYNA-I3012"), (3, "ZRYNA-I3013")]
    {
        let mut raw = seed.clone();
        let function = &mut raw.modules[0].functions[0];
        match mutation {
            0 => function.cleanup_plans[2].actions[0] = raw::DropAction::DropPlace(raw::PlaceId(3)),
            1 => {
                function.cleanup_plans[2].actions[0] =
                    raw::DropAction::DropGenericCloneInitializedPrefix(raw::PlaceId(0));
            }
            2 => function.cleanup_plans[1]
                .actions
                .insert(0, raw::DropAction::DropPlace(raw::PlaceId(3))),
            3 => {
                function.cleanup_plans[2].actions.remove(0);
            }
            _ => unreachable!("four mutations"),
        }
        fixture.rejects(raw, code);
    }
}

#[test]
fn generic_vec_observation_borrow_parameter_cloning_never_fabricates_an_owned_source() {
    let fixture = Fixture::new(Container::Vec, Element::Struct);
    let mut raw = clone_seed(&fixture, raw::BorrowAccess::Shared);
    let function = &mut raw.modules[0].functions[0];
    function.borrow_parameters = vec![raw::BorrowParameter {
        id: raw::BorrowId(0),
        referent: fixture.element,
        access: raw::BorrowAccess::Shared,
        span: function.span,
    }];
    function.blocks[0].instructions.remove(2);
    function.blocks[0].instructions.remove(0);
    function.cleanup_plans.remove(0);
    for (id, plan) in function.cleanup_plans.iter_mut().enumerate() {
        plan.id = raw::CleanupPlanId(u32::try_from(id).expect("three plans"));
    }
    function.blocks[0].instructions[0].kind = raw::InstructionKind::GenericCloneBorrow {
        borrow: raw::BorrowId(0),
        cleanup: raw::CleanupPlanId(0),
        prefix_cleanup: raw::CleanupPlanId(1),
    };
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(5), cleanup: raw::CleanupPlanId(2) };
    let verified = fixture.verify(raw);
    let instruction = verified
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function")
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .next()
        .expect("clone");
    assert!(
        matches!(instruction.generic_clone().expect("borrow clone").source(), super::super::VerifiedGenericCloneSource::Borrow(borrow) if borrow.index() == 0)
    );
    assert_eq!(instruction.place_operands().count(), 0);
}
