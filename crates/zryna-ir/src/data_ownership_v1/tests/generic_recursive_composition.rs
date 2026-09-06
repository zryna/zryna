use super::generic_clone_fixture::{Fixture, GraphKind};
use super::generic_vec_observation::clone_seed;
use super::indexed_borrow_fixture::{Container, Element, Fixture as IndexedFixture};
use super::*;
use zryna_layout::TypeCategory;

fn construction_seed(fixture: &IndexedFixture) -> raw::Program {
    let mut program = fixture.seed(raw::BorrowAccess::Shared);
    let string = raw::TypeId(
        fixture
            .linear
            .types()
            .find(|ty| ty.category() == TypeCategory::String)
            .expect("String")
            .id()
            .index(),
    );
    let function = &mut program.modules[0].functions[0];
    let span = function.span;
    let value = |id, ty| raw::ValueDefinition { id: raw::ValueId(id), ty, span };
    function.parameters = vec![value(0, string), value(1, fixture.integer)];
    function.result = fixture.root;
    function.places = vec![
        raw::Place { id: raw::PlaceId(0), ty: string, span, kind: raw::PlaceKind::Parameter(0) },
        raw::Place {
            id: raw::PlaceId(1),
            ty: fixture.element,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(2)),
        },
        raw::Place {
            id: raw::PlaceId(2),
            ty: fixture.root,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(3)),
        },
    ];
    function.blocks[0].instructions = vec![
        raw::Instruction {
            result: Some(value(2, fixture.element)),
            span,
            kind: raw::InstructionKind::EnumConstruct {
                variant: 0,
                payload: Some(raw::ValueId(0)),
                cleanup: None,
            },
        },
        raw::Instruction {
            result: Some(value(3, fixture.root)),
            span,
            kind: raw::InstructionKind::VecConstruct {
                elements: vec![raw::ValueId(2)],
                cleanup: raw::CleanupPlanId(0),
            },
        },
    ];
    function.cleanup_plans = vec![
        raw::CleanupPlan {
            id: raw::CleanupPlanId(0),
            span,
            actions: vec![raw::DropAction::DropPlace(raw::PlaceId(1))],
        },
        raw::CleanupPlan { id: raw::CleanupPlanId(1), span, actions: vec![] },
    ];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(3), cleanup: raw::CleanupPlanId(1) };
    program
}

fn root_move_seed(fixture: &Fixture) -> raw::Program {
    let mut program = fixture.seed();
    let function = &mut program.modules[0].functions[0];
    function.blocks[0].instructions[0].kind =
        raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(0) };
    function.cleanup_plans = vec![raw::CleanupPlan {
        id: raw::CleanupPlanId(0),
        span: function.span,
        actions: vec![raw::DropAction::DropPlace(raw::PlaceId(1))],
    }];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(3), cleanup: raw::CleanupPlanId(0) };
    program
}

fn root_replacement_seed(fixture: &Fixture) -> raw::Program {
    let mut program = fixture.seed();
    let function = &mut program.modules[0].functions[0];
    function.parameters[1].ty = fixture.root;
    function.places[1].ty = fixture.root;
    function.result = fixture.integer;
    function.blocks[0].instructions[0].kind =
        raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(1) };
    function.blocks[0].instructions.push(raw::Instruction {
        result: None,
        span: function.span,
        kind: raw::InstructionKind::ReplacePlace { place: raw::PlaceId(0), value: raw::ValueId(3) },
    });
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(2), cleanup: raw::CleanupPlanId(0) };
    function.cleanup_plans = vec![raw::CleanupPlan {
        id: raw::CleanupPlanId(0),
        span: function.span,
        actions: vec![raw::DropAction::DropPlace(raw::PlaceId(0))],
    }];
    program
}

fn push_seed(fixture: &IndexedFixture) -> raw::Program {
    let mut program = fixture.seed(raw::BorrowAccess::Shared);
    let function = &mut program.modules[0].functions[0];
    function.blocks[0].instructions = vec![raw::Instruction {
        result: None,
        span: function.span,
        kind: raw::InstructionKind::VecPush {
            vector: raw::PlaceId(0),
            value: raw::ValueId(4),
            cleanup: raw::CleanupPlanId(0),
        },
    }];
    function.cleanup_plans[0].actions = vec![
        raw::DropAction::DropPlace(raw::PlaceId(2)),
        raw::DropAction::DropPlace(raw::PlaceId(1)),
        raw::DropAction::DropPlace(raw::PlaceId(0)),
    ];
    function.cleanup_plans[1].actions = vec![
        raw::DropAction::DropPlace(raw::PlaceId(1)),
        raw::DropAction::DropPlace(raw::PlaceId(0)),
    ];
    program
}

#[test]
fn recursive_construction_rejects_forged_variant_element_and_cleanup_then_recovers() {
    let fixture = IndexedFixture::new(Container::Vec, Element::Recursive);
    let seed = construction_seed(&fixture);
    fixture.verify(seed.clone());

    let mut wrong_variant = seed.clone();
    let raw::InstructionKind::EnumConstruct { variant, .. } =
        &mut wrong_variant.modules[0].functions[0].blocks[0].instructions[0].kind
    else {
        panic!("recursive Enum construction")
    };
    *variant = 9;
    fixture.rejects(wrong_variant, "ZRYNA-I3005");

    let mut wrong_element = seed.clone();
    let raw::InstructionKind::VecConstruct { elements, .. } =
        &mut wrong_element.modules[0].functions[0].blocks[0].instructions[1].kind
    else {
        panic!("recursive Vec construction")
    };
    elements[0] = raw::ValueId(1);
    fixture.rejects(wrong_element, "ZRYNA-I3005");

    let mut missing_cleanup = seed.clone();
    missing_cleanup.modules[0].functions[0].cleanup_plans[0].actions.clear();
    fixture.rejects(missing_cleanup, "ZRYNA-I3012");
    fixture.verify(seed);
}

#[test]
fn recursive_whole_move_and_root_replacement_reject_identity_and_cleanup_forgery() {
    let fixture = Fixture::with_graph(TypeCategory::Vec, GraphKind::Recursive);
    let moved = root_move_seed(&fixture);
    fixture.verify(moved.clone());
    let mut wrong_move = moved.clone();
    wrong_move.modules[0].functions[0].blocks[0].instructions[0]
        .result
        .as_mut()
        .expect("move result")
        .ty = fixture.string;
    fixture.rejects_case(wrong_move, "ZRYNA-I3006", "recursive whole-move result type");
    let mut stale_return = moved;
    stale_return.modules[0].functions[0].cleanup_plans[0]
        .actions
        .push(raw::DropAction::DropPlace(raw::PlaceId(0)));
    fixture.rejects_case(stale_return, "ZRYNA-I3012", "moved recursive root retained");

    let replaced = root_replacement_seed(&fixture);
    fixture.verify(replaced.clone());
    let mut wrong_target = replaced.clone();
    let raw::InstructionKind::ReplacePlace { place, .. } =
        &mut wrong_target.modules[0].functions[0].blocks[0].instructions[1].kind
    else {
        panic!("recursive root replacement")
    };
    *place = raw::PlaceId(1);
    fixture.rejects_case(wrong_target, "ZRYNA-I3010", "wrong recursive root target");
    let mut reused = replaced;
    let duplicate = reused.modules[0].functions[0].blocks[0].instructions[1].clone();
    reused.modules[0].functions[0].blocks[0].instructions.push(duplicate);
    fixture.rejects_case(reused, "ZRYNA-I3010", "reused recursive prepared owner");
}

#[test]
fn recursive_static_move_and_replacement_reject_path_owner_and_cleanup_forgery() {
    let fixture = Fixture::with_graph(TypeCategory::Struct, GraphKind::Recursive);
    let moved = generic_static_transfer::moved(&fixture, true);
    fixture.verify(moved.clone());
    let mut wrong_path = moved.clone();
    let raw::InstructionKind::GenericMoveFromPlace { place } =
        &mut wrong_path.modules[0].functions[0].blocks[0].instructions[0].kind
    else {
        panic!("recursive static move")
    };
    *place = raw::PlaceId(3);
    fixture.rejects_case(wrong_path, "ZRYNA-I3005", "wrong recursive static path type");
    let mut missing_mask = moved;
    missing_mask.modules[0].functions[0].cleanup_plans[0].actions.remove(1);
    fixture.rejects_case(missing_mask, "ZRYNA-I3012", "missing recursive source mask");

    let replaced = generic_static_transfer::replaced(&fixture, true);
    fixture.verify(replaced.clone());
    let mut wrong_value = replaced.clone();
    let raw::InstructionKind::GenericReplacePlace { value, .. } =
        &mut wrong_value.modules[0].functions[0].blocks[0].instructions[1].kind
    else {
        panic!("recursive static replacement")
    };
    *value = raw::ValueId(2);
    fixture.rejects_case(wrong_value, "ZRYNA-I3005", "wrong recursive replacement owner");
    let mut stale_cleanup = replaced;
    stale_cleanup.modules[0].functions[0].cleanup_plans[2].actions.swap(0, 1);
    fixture.rejects_case(stale_cleanup, "ZRYNA-I3012", "recursive replacement cleanup order");
}

#[test]
fn recursive_vec_observation_replacement_and_push_reject_forged_authority() {
    let fixture = IndexedFixture::new(Container::Vec, Element::Recursive);
    let observed = clone_seed(&fixture, raw::BorrowAccess::Shared);
    fixture.verify(observed.clone());
    let mut foreign = observed.clone();
    let raw::InstructionKind::GenericCloneBorrow { borrow, .. } =
        &mut foreign.modules[0].functions[0].blocks[0].instructions[1].kind
    else {
        panic!("recursive indexed clone")
    };
    *borrow = raw::BorrowId(99);
    fixture.rejects(foreign, "ZRYNA-I3005");
    let mut wrong_prefix = observed;
    wrong_prefix.modules[0].functions[0].cleanup_plans[2].actions[0] =
        raw::DropAction::DropGenericCloneInitializedPrefix(raw::PlaceId(0));
    fixture.rejects(wrong_prefix, "ZRYNA-I3012");

    let mut replaced = clone_seed(&fixture, raw::BorrowAccess::Exclusive);
    let function = &mut replaced.modules[0].functions[0];
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
    fixture.verify(replaced.clone());
    let mut wrong_commit = replaced;
    let raw::InstructionKind::BorrowReplace { value, .. } =
        &mut wrong_commit.modules[0].functions[0].blocks[0].instructions[2].kind
    else {
        panic!("recursive indexed replacement")
    };
    *value = raw::ValueId(2);
    fixture.rejects(wrong_commit, "ZRYNA-I3005");

    let pushed = push_seed(&fixture);
    fixture.verify(pushed.clone());
    let mut wrong_push = pushed.clone();
    let raw::InstructionKind::VecPush { value, .. } =
        &mut wrong_push.modules[0].functions[0].blocks[0].instructions[0].kind
    else {
        panic!("recursive Vec push")
    };
    *value = raw::ValueId(2);
    fixture.rejects(wrong_push, "ZRYNA-I3005");
    let mut wrong_order = pushed;
    wrong_order.modules[0].functions[0].cleanup_plans[0].actions.swap(0, 1);
    fixture.rejects(wrong_order, "ZRYNA-I3012");
}

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
