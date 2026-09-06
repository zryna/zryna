use super::*;
use crate::data_ownership_v1::{BorrowIdentity, VerifiedCallArgument};

fn caller_places(span: zryna_source::Span) -> Vec<raw::Place> {
    [
        raw::PlaceKind::Temporary(raw::ValueId(0)),
        raw::PlaceKind::Local(0),
        raw::PlaceKind::Temporary(raw::ValueId(1)),
        raw::PlaceKind::Local(1),
        raw::PlaceKind::Temporary(raw::ValueId(2)),
    ]
    .into_iter()
    .enumerate()
    .map(|(id, kind)| raw::Place {
        id: raw::PlaceId(u32::try_from(id).expect("five places")),
        ty: raw::TypeId(2),
        span,
        kind,
    })
    .collect()
}

fn borrowed_clone_callee(span: zryna_source::Span) -> raw::Function {
    raw::Function {
        id: raw::FunctionId { module: raw::ModuleId(0), declaration: 1 },
        entry_export: None,
        span,
        parameters: vec![],
        borrow_parameters: vec![raw::BorrowParameter {
            id: raw::BorrowId(0),
            referent: raw::TypeId(2),
            access: raw::BorrowAccess::Shared,
            span,
        }],
        result: raw::TypeId(2),
        places: vec![raw::Place {
            id: raw::PlaceId(0),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(0)),
        }],
        blocks: vec![raw::Block {
            id: raw::BlockId(0),
            parameters: vec![],
            instructions: vec![raw::Instruction {
                result: Some(raw::ValueDefinition {
                    id: raw::ValueId(0),
                    ty: raw::TypeId(2),
                    span,
                }),
                span,
                kind: raw::InstructionKind::GenericCloneBorrow {
                    borrow: raw::BorrowId(0),
                    cleanup: raw::CleanupPlanId(0),
                    prefix_cleanup: raw::CleanupPlanId(1),
                },
            }],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Return {
                    value: raw::ValueId(0),
                    cleanup: raw::CleanupPlanId(2),
                },
            }],
        }],
        cleanup_plans: vec![
            raw::CleanupPlan { id: raw::CleanupPlanId(0), span, actions: vec![] },
            raw::CleanupPlan {
                id: raw::CleanupPlanId(1),
                span,
                actions: vec![raw::DropAction::DropGenericCloneInitializedPrefix(raw::PlaceId(0))],
            },
            raw::CleanupPlan { id: raw::CleanupPlanId(2), span, actions: vec![] },
        ],
    }
}

fn scoped_call_program(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
) -> raw::Program {
    let mut program = string_literal_program(sources, linear, linux, b"owner".to_vec());
    let caller = &mut program.modules[0].functions[0];
    let span = caller.span;
    caller.places = caller_places(span);
    caller.blocks[0].instructions.extend([
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::InitializePlace {
                place: raw::PlaceId(1),
                value: raw::ValueId(0),
            },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::BeginBorrow(raw::BorrowDefinition {
                id: raw::BorrowId(0),
                place: raw::PlaceId(1),
                access: raw::BorrowAccess::Shared,
                span,
            }),
        },
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(2), span }),
            span,
            kind: raw::InstructionKind::DirectCall {
                callee: raw::FunctionId { module: raw::ModuleId(0), declaration: 1 },
                arguments: vec![raw::CallArgument::Borrow(raw::BorrowId(0))],
                cleanup: raw::CleanupPlanId(1),
            },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::InitializePlace {
                place: raw::PlaceId(3),
                value: raw::ValueId(1),
            },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(3) },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::EndBorrow { borrow: raw::BorrowId(0) },
        },
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(2), span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(1) },
        },
    ]);
    caller.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(2), cleanup: raw::CleanupPlanId(2) };
    caller.cleanup_plans = vec![
        raw::CleanupPlan { id: raw::CleanupPlanId(0), span, actions: vec![] },
        raw::CleanupPlan {
            id: raw::CleanupPlanId(1),
            span,
            actions: vec![raw::DropAction::DropPlace(raw::PlaceId(1))],
        },
        raw::CleanupPlan { id: raw::CleanupPlanId(2), span, actions: vec![] },
    ];
    program.modules[0].functions.push(borrowed_clone_callee(span));
    program
}

#[test]
fn nonindexed_lexical_call_is_verified_as_nonescaping_authority() {
    let (sources, linear, linux) = authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    for _ in 0..2 {
        let verified = verify(
            scoped_call_program(&sources, &linear, &linux),
            &sources,
            entry,
            linear.clone(),
            linux.clone(),
        )
        .expect("verified lexical borrowed call");
        let caller = verified.modules().next().expect("module").functions().next().expect("caller");
        let call = caller
            .blocks()
            .next()
            .expect("block")
            .instructions()
            .find(|instruction| instruction.kind() == VerifiedInstructionKind::DirectCall)
            .expect("direct call");
        assert_eq!(
            call.call_arguments().collect::<Vec<_>>(),
            [VerifiedCallArgument::Borrow(call.failure_ended_borrows().next().expect("borrow"))]
        );
        assert_eq!(
            call.failure_ended_borrows().map(BorrowIdentity::index).collect::<Vec<_>>(),
            [0]
        );
        assert_eq!(call.derived_drop_actions().count(), 1);
    }
}

#[test]
fn nonindexed_call_rejects_inactive_wrong_region_and_escape_then_recovers() {
    let (sources, linear, linux) = authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let seed = scoped_call_program(&sources, &linear, &linux);
    let mut inactive = seed.clone();
    inactive.modules[0].functions[0].blocks[0].instructions.swap(2, 3);
    let mut wrong_region = seed.clone();
    let raw::InstructionKind::BeginBorrow(definition) =
        &mut wrong_region.modules[0].functions[0].blocks[0].instructions[2].kind
    else {
        panic!("lexical begin")
    };
    definition.place = raw::PlaceId(0);
    let mut escaping = seed.clone();
    escaping.modules[0].functions[0].blocks[0].instructions.remove(6);
    for raw in [inactive, wrong_region, escaping] {
        for _ in 0..2 {
            let diagnostics = verify(raw.clone(), &sources, entry, linear.clone(), linux.clone())
                .expect_err("hostile borrowed call");
            assert!(
                diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3011"),
                "expected borrow rejection: {diagnostics:?}"
            );
        }
    }
    verify(seed, &sources, entry, linear, linux).expect("recovery after hostile programs");
}
