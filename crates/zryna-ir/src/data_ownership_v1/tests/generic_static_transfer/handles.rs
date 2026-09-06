use super::*;
use crate::data_ownership_v1::tests::generic_clone_fixture::GraphKind;
use crate::data_ownership_v1::{checked_add, preflight};

fn replacement(fixture: &Fixture, nested: bool) -> raw::Program {
    let mut program = replaced(fixture, nested);
    program.modules[0].functions[0].blocks[0].instructions[0].kind =
        raw::InstructionKind::HandleAwareClonePlace {
            place: raw::PlaceId(if nested { 4 } else { 3 }),
            cleanup: raw::CleanupPlanId(0),
            prefix_cleanup: raw::CleanupPlanId(1),
        };
    program
}

#[test]
fn handle_static_transfer_without_clone_preserves_recursive_masks_and_owner_identity() {
    for mode in [GraphKind::Shared, GraphKind::Weak, GraphKind::RecursiveShared] {
        let fixture = Fixture::with_graph(TypeCategory::Struct, mode);
        for nested in [false, true] {
            let raw = moved(&fixture, nested);
            for _ in 0..2 {
                let verified = fixture.verify(raw.clone());
                let function = verified
                    .modules()
                    .next()
                    .expect("module")
                    .functions()
                    .next()
                    .expect("function");
                let block = function.blocks().next().expect("block");
                let operation = block.instructions().next().expect("move");
                assert_eq!(block.instructions().len(), 1, "no clone activates eligibility");
                assert_eq!(operation.kind(), VerifiedInstructionKind::GenericMoveFromPlace);
                assert_eq!(operation.result().expect("result").index(), 3);
                let target = operation.place_operands().next().expect("target");
                assert_eq!(target.index(), if nested { 4 } else { 3 });
                let drops = block.terminator().derived_drop_actions().collect::<Vec<_>>();
                assert_eq!(
                    drops.iter().map(|drop| drop.root().index()).collect::<Vec<_>>(),
                    [1, 0]
                );
                assert_eq!(drops[1].moved_projections().collect::<Vec<_>>(), [target]);
                assert_eq!(operation.derived_drop_actions().len(), 0);
            }
            fixture.rejects_case(
                projected(&fixture, nested),
                "ZRYNA-I3005",
                "generic clone still excludes handles",
            );
            fixture.verify(raw);
        }
    }
}

#[test]
fn handle_static_transfer_replacement_retains_old_target_through_count_failure() {
    for mode in [GraphKind::Shared, GraphKind::Weak] {
        let fixture = Fixture::with_graph(TypeCategory::Struct, mode);
        for nested in [false, true] {
            for _ in 0..2 {
                let verified = fixture.verify(replacement(&fixture, nested));
                let function = verified
                    .modules()
                    .next()
                    .expect("module")
                    .functions()
                    .next()
                    .expect("function");
                let block = function.blocks().next().expect("block");
                let operations = block.instructions().collect::<Vec<_>>();
                let clone = operations[0].handle_aware_clone().expect("explicit handle recipe");
                let pending = operations[0].derived_drop_actions().collect::<Vec<_>>();
                assert_eq!(
                    pending.iter().map(|drop| drop.root().index()).collect::<Vec<_>>(),
                    [1, 0]
                );
                assert!(pending.iter().all(|drop| drop.moved_projections().len() == 0));
                let prefix = operations[0]
                    .handle_aware_clone_prefix_failure_drop_actions()
                    .collect::<Vec<_>>();
                assert_eq!(prefix[0].root(), clone.destination());
                assert_eq!(&prefix[1..], pending.as_slice());
                let commit = operations[1];
                assert_eq!(commit.value_operands().collect::<Vec<_>>(), [clone.result()]);
                assert_eq!(commit.kind(), VerifiedInstructionKind::GenericReplacePlace);
                assert_eq!(
                    commit.derived_drop_actions().map(|drop| drop.root()).collect::<Vec<_>>(),
                    commit.place_operands().collect::<Vec<_>>()
                );
                assert_eq!(block.terminator().derived_drop_actions().collect::<Vec<_>>(), pending);
            }
        }
    }
}

#[test]
fn handle_static_transfer_rejects_exact_type_borrow_and_consumption_forgery_then_recovers() {
    for mode in [GraphKind::Shared, GraphKind::Weak] {
        let fixture = Fixture::with_graph(TypeCategory::Struct, mode);
        for nested in [false, true] {
            let seed = replacement(&fixture, nested);
            let mut wrong = seed.clone();
            let function = &mut wrong.modules[0].functions[0];
            let raw::InstructionKind::GenericReplacePlace { value, .. } =
                &mut function.blocks[0].instructions[1].kind
            else {
                unreachable!()
            };
            *value = raw::ValueId(1);
            fixture.rejects_case(
                wrong,
                "ZRYNA-I3005",
                "String cannot replace the handle container",
            );
            for moving in [false, true] {
                let mut borrowed = if moving { moved(&fixture, nested) } else { seed.clone() };
                let function = &mut borrowed.modules[0].functions[0];
                function.blocks[0].instructions.insert(
                    usize::from(!moving),
                    begin_borrow(0, 0, raw::BorrowAccess::Shared, function.span),
                );
                function.blocks[0].instructions.push(end_borrow(0, function.span));
                fixture.rejects_case(
                    borrowed,
                    "ZRYNA-I3010",
                    "live enclosing borrow excludes transfer",
                );
            }
            let mut reused = seed.clone();
            let instructions = &mut reused.modules[0].functions[0].blocks[0].instructions;
            instructions.push(instructions[1].clone());
            fixture.rejects_case(reused, "ZRYNA-I3010", "RHS owner consumed exactly once");
            let mut hole = seed.clone();
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
                    result: Some(raw::ValueDefinition {
                        id: raw::ValueId(4),
                        ty,
                        span: function.span,
                    }),
                    span: function.span,
                    kind: raw::InstructionKind::GenericMoveFromPlace { place: target },
                },
            );
            function.cleanup_plans[2].actions.insert(0, raw::DropAction::DropPlace(temporary));
            fixture.rejects_case(hole, "ZRYNA-I3010", "replacement cannot repair moved target");
            fixture.verify(seed);
        }
    }
}

#[test]
fn handle_static_transfer_rejects_repeated_move_forged_path_and_omitted_parent_cleanup() {
    let fixture = Fixture::with_graph(TypeCategory::Struct, GraphKind::Shared);
    for nested in [false, true] {
        let seed = moved(&fixture, nested);
        let mut repeated = seed.clone();
        let function = &mut repeated.modules[0].functions[0];
        let mut second = function.blocks[0].instructions[0].clone();
        second.result.as_mut().expect("move result").id = raw::ValueId(4);
        let temporary = raw::PlaceId(u32::try_from(function.places.len()).expect("temporary"));
        function.places.push(raw::Place {
            id: temporary,
            ty: function.result,
            span: function.span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(4)),
        });
        function.blocks[0].instructions.push(second);
        function.cleanup_plans[0].actions.insert(0, raw::DropAction::DropPlace(temporary));
        fixture.rejects_case(repeated, "ZRYNA-I3010", "subtree cannot move twice");
        let mut forged = seed.clone();
        forged.modules[0].functions[0].places[3].kind =
            raw::PlaceKind::StructField { base: raw::PlaceId(0), ordinal: 1 };
        fixture.rejects_case(forged, "ZRYNA-I3006", "projection type cannot forge another field");
        let mut omitted = seed.clone();
        omitted.modules[0].functions[0].cleanup_plans[0].actions.pop();
        fixture.rejects_case(omitted, "ZRYNA-I3012", "retained parent still requires cleanup");
        fixture.verify(seed);
    }
}

#[test]
fn handle_static_transfer_rejects_partial_target_and_wrong_move_result() {
    let fixture = Fixture::with_graph(TypeCategory::Struct, GraphKind::Shared);
    let seed = replacement(&fixture, false);
    let mut partial = seed.clone();
    let function = &mut partial.modules[0].functions[0];
    let array =
        fixture.linear.types().find(|ty| ty.category() == TypeCategory::FixedArray).expect("array");
    let ty = raw::TypeId(array.referenced_type().expect("element").index());
    function.places.extend([
        raw::Place {
            id: raw::PlaceId(4),
            ty,
            span: function.span,
            kind: raw::PlaceKind::FixedArrayConstant { base: raw::PlaceId(3), index: 0 },
        },
        raw::Place {
            id: raw::PlaceId(5),
            ty,
            span: function.span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(4)),
        },
    ]);
    function.blocks[0].instructions.insert(
        1,
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(4), ty, span: function.span }),
            span: function.span,
            kind: raw::InstructionKind::GenericMoveFromPlace { place: raw::PlaceId(4) },
        },
    );
    function.cleanup_plans[2].actions.insert(0, raw::DropAction::DropPlace(raw::PlaceId(5)));
    fixture.rejects_case(
        partial,
        "ZRYNA-I3010",
        "complete replacement cannot repair a partial array",
    );
    let mut wrong = moved(&fixture, false);
    let function = &mut wrong.modules[0].functions[0];
    function.result = fixture.root;
    function.places[2].ty = fixture.root;
    function.blocks[0].instructions[0].result.as_mut().expect("move").ty = fixture.root;
    fixture.rejects_case(wrong, "ZRYNA-I3005", "subobject move has exact result type");
    fixture.verify(seed);
}

#[test]
fn handle_static_transfer_resource_preflight_exact_extra_overflow_and_recovery() {
    let fixture = Fixture::with_graph(TypeCategory::Struct, GraphKind::Weak);
    let seed = replacement(&fixture, true);
    fixture.verify(seed.clone());
    let mut program = seed.clone();
    let instruction = program.modules[0].functions[0].blocks[0].instructions[1].clone();
    // Counter-only controls exercise preflight, not a valid repeated-replacement program.
    program.modules[0].functions[0].blocks[0]
        .instructions
        .resize(MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION, instruction.clone());
    let mut exact = Errors::default();
    preflight(&program, &fixture.linear, &mut exact);
    assert!(exact.is_empty());
    program.modules[0].functions[0].blocks[0].instructions.push(instruction);
    let mut extra = Errors::default();
    preflight(&program, &fixture.linear, &mut extra);
    let first = extra.finish();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].code(), "ZRYNA-I3201");
    let mut replay = Errors::default();
    preflight(&program, &fixture.linear, &mut replay);
    assert_eq!(first, replay.finish());
    let mut overflow = Errors::default();
    assert_eq!(checked_add(usize::MAX, 1, "static transfer count", &mut overflow), usize::MAX);
    assert_eq!(overflow.finish()[0].code(), "ZRYNA-I3201");
    fixture.verify(seed);
}
