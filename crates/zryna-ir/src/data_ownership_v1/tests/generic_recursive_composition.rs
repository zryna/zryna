use super::generic_clone_fixture::{Fixture, GraphKind};
use super::*;
use zryna_layout::TypeCategory;

#[test]
fn recursive_clone_rejects_wrong_identity_prefix_and_cleanup_order_then_recovers() {
    let fixture = Fixture::with_graph(TypeCategory::Vec, GraphKind::Recursive);
    let seed = fixture.seed();
    fixture.verify(seed.clone());

    let mut wrong_source = seed.clone();
    let function = &mut wrong_source.modules[0].functions[0];
    let raw::InstructionKind::GenericClonePlace { place, .. } =
        &mut function.blocks[0].instructions[0].kind
    else {
        panic!("recursive clone");
    };
    *place = raw::PlaceId(1);
    fixture.rejects_case(wrong_source, "ZRYNA-I3005", "wrong recursive source identity");

    let mut wrong_destination = seed.clone();
    wrong_destination.modules[0].functions[0].places[2].kind =
        raw::PlaceKind::Temporary(raw::ValueId(99));
    fixture.rejects_case(wrong_destination, "ZRYNA-I3006", "wrong recursive destination identity");

    let mut wrong_prefix = seed.clone();
    wrong_prefix.modules[0].functions[0].cleanup_plans[1].actions[0] =
        raw::DropAction::DropGenericCloneInitializedPrefix(raw::PlaceId(0));
    fixture.rejects_case(wrong_prefix, "ZRYNA-I3012", "source used as destination prefix");

    let mut wrong_order = seed.clone();
    wrong_order.modules[0].functions[0].cleanup_plans[1].actions.swap(1, 2);
    fixture.rejects_case(wrong_order, "ZRYNA-I3013", "recursive survivor order");
    fixture.verify(seed);
}

#[test]
fn recursive_clone_resource_preflight_is_exact_checked_and_replay_stable() {
    let fixture = Fixture::with_graph(TypeCategory::Vec, GraphKind::Recursive);
    let seed = fixture.seed();
    fixture.verify(seed.clone());
    let mut program = seed.clone();
    let template = program.modules[0].functions[0].cleanup_plans[0].clone();
    program.modules[0].functions[0]
        .cleanup_plans
        .resize(MAX_CLEANUP_PLANS_PER_FUNCTION, template.clone());
    let mut exact = Errors::default();
    super::super::preflight(&program, &fixture.linear, &mut exact);
    assert!(exact.is_empty());

    program.modules[0].functions[0].cleanup_plans.push(template);
    let check = || {
        let mut errors = Errors::default();
        super::super::preflight(&program, &fixture.linear, &mut errors);
        errors.finish()
    };
    let first = check();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].code(), "ZRYNA-I3201");
    assert_eq!(first, check());

    let mut overflow = Errors::default();
    assert_eq!(
        super::super::checked_add(usize::MAX, 1, "recursive clone cleanup count", &mut overflow),
        usize::MAX
    );
    assert_eq!(overflow.finish()[0].code(), "ZRYNA-I3201");
    fixture.verify(seed);
}
