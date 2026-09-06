use super::generic_vec_observation::clone_seed;
use super::indexed_borrow_fixture::{Container, Element, Fixture};
use super::*;

fn seed(element: Element) -> (Fixture, raw::Program) {
    let fixture = Fixture::new(Container::Vec, element);
    let mut program = clone_seed(&fixture, raw::BorrowAccess::Shared);
    let raw::InstructionKind::GenericCloneBorrow { borrow, cleanup, prefix_cleanup } =
        program.modules[0].functions[0].blocks[0].instructions[1].kind
    else {
        panic!("generic Vec clone")
    };
    program.modules[0].functions[0].blocks[0].instructions[1].kind =
        raw::InstructionKind::HandleAwareCloneBorrow { borrow, cleanup, prefix_cleanup };
    (fixture, program)
}

#[test]
fn generic_vec_handle_ir_rejects_stale_or_foreign_borrow_then_recovers() {
    for element in [Element::Shared, Element::Weak] {
        let (fixture, seed) = seed(element);
        fixture.verify(seed.clone());

        let mut stale = seed.clone();
        stale.modules[0].functions[0].blocks[0].instructions.swap(1, 2);
        fixture.rejects(stale.clone(), "ZRYNA-I3011");
        fixture.rejects(stale, "ZRYNA-I3011");

        let mut foreign = seed.clone();
        let raw::InstructionKind::HandleAwareCloneBorrow { borrow, .. } =
            &mut foreign.modules[0].functions[0].blocks[0].instructions[1].kind
        else {
            panic!("handle clone")
        };
        *borrow = raw::BorrowId(99);
        fixture.rejects(foreign.clone(), "ZRYNA-I3005");
        fixture.rejects(foreign, "ZRYNA-I3005");

        fixture.verify(seed);
    }
}

#[test]
fn generic_vec_handle_ir_rejects_wrong_result_and_cleanup_identity_then_recovers() {
    for element in [Element::Shared, Element::Weak] {
        let (fixture, seed) = seed(element);
        let mut wrong_type = seed.clone();
        let function = &mut wrong_type.modules[0].functions[0];
        function.result = fixture.integer;
        function.places[3].ty = fixture.integer;
        function.blocks[0].instructions[1]
            .result
            .as_mut()
            .expect("verified generic Vec handle shape")
            .ty = fixture.integer;
        fixture.rejects(wrong_type.clone(), "ZRYNA-I3005");
        fixture.rejects(wrong_type, "ZRYNA-I3005");

        let mut missing = seed.clone();
        missing.modules[0].functions[0].cleanup_plans[2].actions.remove(0);
        fixture.rejects(missing.clone(), "ZRYNA-I3013");
        fixture.rejects(missing, "ZRYNA-I3013");

        let mut duplicate = seed.clone();
        let action = duplicate.modules[0].functions[0].cleanup_plans[2].actions[0];
        duplicate.modules[0].functions[0].cleanup_plans[2].actions.insert(0, action);
        fixture.rejects(duplicate.clone(), "ZRYNA-I3012");
        fixture.rejects(duplicate, "ZRYNA-I3012");

        fixture.verify(seed);
    }
}
