use super::generic_clone_fixture::Fixture;
use super::*;
use zryna_layout::TypeCategory;

fn projected(fixture: &Fixture, nested: bool) -> raw::Program {
    let mut raw = fixture.seed();
    let array =
        fixture.linear.types().find(|ty| ty.category() == TypeCategory::FixedArray).expect("array");
    let array_ty = raw::TypeId(array.id().index());
    let element = if nested {
        raw::TypeId(array.referenced_type().expect("Vec element").index())
    } else {
        array_ty
    };
    let function = &mut raw.modules[0].functions[0];
    function.result = element;
    function.places[2].ty = element;
    function.places.push(raw::Place {
        id: raw::PlaceId(3),
        ty: array_ty,
        span: function.span,
        kind: raw::PlaceKind::StructField { base: raw::PlaceId(0), ordinal: 0 },
    });
    if nested {
        function.places.push(raw::Place {
            id: raw::PlaceId(4),
            ty: element,
            span: function.span,
            kind: raw::PlaceKind::FixedArrayConstant { base: raw::PlaceId(3), index: 0 },
        });
    }
    function.blocks[0].instructions[0].result.as_mut().expect("result").ty = element;
    function.blocks[0].instructions[0].kind = raw::InstructionKind::GenericClonePlace {
        place: raw::PlaceId(if nested { 4 } else { 3 }),
        cleanup: raw::CleanupPlanId(0),
        prefix_cleanup: raw::CleanupPlanId(1),
    };
    raw
}

fn moved(fixture: &Fixture, nested: bool) -> raw::Program {
    let mut raw = projected(fixture, nested);
    let function = &mut raw.modules[0].functions[0];
    function.blocks[0].instructions[0].kind = raw::InstructionKind::GenericMoveFromPlace {
        place: raw::PlaceId(if nested { 4 } else { 3 }),
    };
    function.cleanup_plans.truncate(1);
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(3), cleanup: raw::CleanupPlanId(0) };
    raw
}

fn replaced(fixture: &Fixture, nested: bool) -> raw::Program {
    let mut raw = projected(fixture, nested);
    let function = &mut raw.modules[0].functions[0];
    function.result = fixture.integer;
    function.blocks[0].instructions.push(raw::Instruction {
        result: None,
        span: function.span,
        kind: raw::InstructionKind::GenericReplacePlace {
            place: raw::PlaceId(if nested { 4 } else { 3 }),
            value: raw::ValueId(3),
        },
    });
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(2), cleanup: raw::CleanupPlanId(2) };
    raw
}

#[test]
fn generic_static_transfer_move_derives_one_exact_recursive_subtree_mask() {
    let fixture = Fixture::new(TypeCategory::Struct);
    for nested in [false, true] {
        for _ in 0..2 {
            let verified = fixture.verify(moved(&fixture, nested));
            let function =
                verified.modules().next().expect("module").functions().next().expect("function");
            let block = function.blocks().next().expect("block");
            let instruction = block.instructions().next().expect("move");
            assert_eq!(instruction.kind(), VerifiedInstructionKind::GenericMoveFromPlace);
            let projection = instruction.place_operands().next().expect("projection");
            assert_eq!(projection.index(), if nested { 4 } else { 3 });
            let actions = block.terminator().derived_drop_actions().collect::<Vec<_>>();
            assert_eq!(actions.iter().map(|a| a.root().index()).collect::<Vec<_>>(), [1, 0]);
            assert_eq!(actions[1].moved_projections().collect::<Vec<_>>(), [projection]);
            assert_eq!(instruction.derived_drop_actions().len(), 0);
        }
    }
}

#[test]
fn generic_static_transfer_clone_then_replace_retains_source_until_exact_commit() {
    let fixture = Fixture::new(TypeCategory::Struct);
    for nested in [false, true] {
        for _ in 0..2 {
            let verified = fixture.verify(replaced(&fixture, nested));
            let function =
                verified.modules().next().expect("module").functions().next().expect("function");
            let block = function.blocks().next().expect("block");
            let instructions = block.instructions().collect::<Vec<_>>();
            let clone = instructions[0].generic_clone().expect("prepared clone");
            let pending = instructions[0].derived_drop_actions().collect::<Vec<_>>();
            assert_eq!(pending.iter().map(|a| a.root().index()).collect::<Vec<_>>(), [1, 0]);
            assert!(pending.iter().all(|a| a.moved_projections().len() == 0));
            let prefix =
                instructions[0].generic_clone_prefix_failure_drop_actions().collect::<Vec<_>>();
            assert_eq!(prefix[0].root(), clone.destination());
            assert_eq!(&prefix[1..], pending.as_slice());
            let commit = instructions[1];
            assert_eq!(commit.kind(), VerifiedInstructionKind::GenericReplacePlace);
            assert_eq!(commit.value_operands().collect::<Vec<_>>(), [clone.result()]);
            let target = commit.place_operands().next().expect("exact target");
            assert_eq!(
                commit.derived_drop_actions().map(|a| a.root()).collect::<Vec<_>>(),
                [target]
            );
            assert_eq!(block.terminator().derived_drop_actions().collect::<Vec<_>>(), pending);
        }
    }
}

#[test]
fn generic_static_transfer_rejects_wrong_referent_and_unavailable_move() {
    let fixture = Fixture::new(TypeCategory::Struct);
    for nested in [false, true] {
        let seed = moved(&fixture, nested);
        fixture.verify(seed.clone());
        let mut wrong = seed.clone();
        let function = &mut wrong.modules[0].functions[0];
        function.result = fixture.root;
        function.places[2].ty = fixture.root;
        function.blocks[0].instructions[0].result.as_mut().expect("result").ty = fixture.root;
        fixture.rejects_case(wrong, "ZRYNA-I3005", "static move cannot return enclosing root type");
        let mut borrowed = seed;
        let function = &mut borrowed.modules[0].functions[0];
        function.blocks[0]
            .instructions
            .insert(0, begin_borrow(0, 0, raw::BorrowAccess::Shared, function.span));
        function.blocks[0].instructions.push(end_borrow(0, function.span));
        fixture.rejects_case(
            borrowed,
            "ZRYNA-I3010",
            "static move cannot consume borrowed enclosing owner",
        );
    }
}

#[test]
fn generic_static_transfer_rejects_wrong_replacement_type_and_borrowed_target() {
    let fixture = Fixture::new(TypeCategory::Struct);
    for nested in [false, true] {
        let seed = replaced(&fixture, nested);
        fixture.verify(seed.clone());
        let mut wrong = seed.clone();
        let function = &mut wrong.modules[0].functions[0];
        let raw::InstructionKind::GenericReplacePlace { value, .. } =
            &mut function.blocks[0].instructions[1].kind
        else {
            panic!("commit");
        };
        *value = raw::ValueId(1);
        fixture.rejects_case(wrong, "ZRYNA-I3005", "static replacement requires exact RHS type");
        let mut borrowed = seed;
        let function = &mut borrowed.modules[0].functions[0];
        function.blocks[0]
            .instructions
            .insert(1, begin_borrow(0, 0, raw::BorrowAccess::Shared, function.span));
        function.blocks[0].instructions.push(end_borrow(0, function.span));
        fixture.rejects_case(
            borrowed,
            "ZRYNA-I3010",
            "static replacement cannot mutate borrowed enclosing owner",
        );
    }
}

#[test]
fn generic_static_transfer_rejects_reused_rhs_and_a_hole_at_the_commit_target() {
    let fixture = Fixture::new(TypeCategory::Struct);
    for nested in [false, true] {
        let seed = replaced(&fixture, nested);
        fixture.verify(seed.clone());
        let mut reused = seed.clone();
        let instructions = &mut reused.modules[0].functions[0].blocks[0].instructions;
        instructions.push(instructions[1].clone());
        fixture.rejects_case(reused, "ZRYNA-I3010", "replacement RHS owner transfers exactly once");
        let mut hole = seed;
        let function = &mut hole.modules[0].functions[0];
        let target = raw::PlaceId(if nested { 4 } else { 3 });
        let ty = function.places[target.0 as usize].ty;
        let temporary = raw::PlaceId(u32::try_from(function.places.len()).expect("temporary"));
        function.places.push(raw::Place {
            id: temporary,
            ty,
            span: function.span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(4)),
        });
        function.blocks[0].instructions.insert(
            1,
            raw::Instruction {
                result: Some(raw::ValueDefinition { id: raw::ValueId(4), ty, span: function.span }),
                span: function.span,
                kind: raw::InstructionKind::GenericMoveFromPlace { place: target },
            },
        );
        function.cleanup_plans[2].actions.insert(0, raw::DropAction::DropPlace(temporary));
        fixture.rejects_case(hole, "ZRYNA-I3010", "replacement cannot repair a moved target");
    }
}
