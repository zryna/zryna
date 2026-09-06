use super::active_enum_payload_borrow::fixture::{Mode, Seed};
use super::*;
use crate::data_ownership_v1::{BorrowIdentity, VerifiedCallArgument};

#[derive(Clone, Copy)]
enum StaticShape {
    Struct,
    FixedArray,
}

fn clone_callee(
    span: zryna_source::Span,
    referent: raw::TypeId,
    access: raw::BorrowAccess,
) -> raw::Function {
    raw::Function {
        id: raw::FunctionId { module: raw::ModuleId(0), declaration: 1 },
        entry_export: None,
        span,
        parameters: vec![],
        borrow_parameters: vec![raw::BorrowParameter {
            id: raw::BorrowId(0),
            referent,
            access,
            span,
        }],
        result: referent,
        places: vec![raw::Place {
            id: raw::PlaceId(0),
            ty: referent,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(0)),
        }],
        blocks: vec![raw::Block {
            id: raw::BlockId(0),
            parameters: vec![],
            instructions: vec![raw::Instruction {
                result: Some(raw::ValueDefinition { id: raw::ValueId(0), ty: referent, span }),
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

fn static_call_program(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
    aggregate: raw::TypeId,
    shape: StaticShape,
    access: raw::BorrowAccess,
) -> raw::Program {
    let mut raw = program(sources, linear, linux);
    raw.modules[0].data_declarations = 2;
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    function.entry_export = None;
    function.parameters = vec![raw::ValueDefinition { id: raw::ValueId(0), ty: aggregate, span }];
    function.result = aggregate;
    let projection = match shape {
        StaticShape::Struct => raw::PlaceKind::StructField { base: raw::PlaceId(0), ordinal: 1 },
        StaticShape::FixedArray => {
            raw::PlaceKind::FixedArrayConstant { base: raw::PlaceId(0), index: 1 }
        }
    };
    function.places = vec![
        raw::Place { id: raw::PlaceId(0), ty: aggregate, span, kind: raw::PlaceKind::Parameter(0) },
        raw::Place { id: raw::PlaceId(1), ty: raw::TypeId(2), span, kind: projection },
        raw::Place {
            id: raw::PlaceId(2),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(1)),
        },
        raw::Place {
            id: raw::PlaceId(3),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Local(0),
        },
        raw::Place {
            id: raw::PlaceId(4),
            ty: aggregate,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(2)),
        },
    ];
    function.blocks[0].instructions = call_sequence(span, aggregate, access, raw::PlaceId(1));
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(2), cleanup: raw::CleanupPlanId(1) };
    function.cleanup_plans = caller_cleanup(span, raw::PlaceId(0));
    raw.modules[0].functions.push(clone_callee(span, raw::TypeId(2), access));
    raw
}

fn call_sequence(
    span: zryna_source::Span,
    owner: raw::TypeId,
    access: raw::BorrowAccess,
    borrowed_place: raw::PlaceId,
) -> Vec<raw::Instruction> {
    vec![
        begin_borrow(0, borrowed_place.0, access, span),
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(2), span }),
            span,
            kind: raw::InstructionKind::DirectCall {
                callee: raw::FunctionId { module: raw::ModuleId(0), declaration: 1 },
                arguments: vec![raw::CallArgument::Borrow(raw::BorrowId(0))],
                cleanup: raw::CleanupPlanId(0),
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
        end_borrow(0, span),
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(2), ty: owner, span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(0) },
        },
    ]
}

fn caller_cleanup(span: zryna_source::Span, owner: raw::PlaceId) -> Vec<raw::CleanupPlan> {
    vec![
        raw::CleanupPlan {
            id: raw::CleanupPlanId(0),
            span,
            actions: vec![raw::DropAction::DropPlace(owner)],
        },
        raw::CleanupPlan { id: raw::CleanupPlanId(1), span, actions: vec![] },
    ]
}

fn assert_call_scope(
    function: super::super::VerifiedFunction<'_>,
    projection: u32,
    post_end_kind: VerifiedInstructionKind,
    failure_drop_count: usize,
) {
    let instructions = function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
    let begin_index = instructions
        .iter()
        .position(|item| item.kind() == VerifiedInstructionKind::BeginBorrow)
        .expect("begin borrow");
    let call_index = instructions
        .iter()
        .position(|item| item.kind() == VerifiedInstructionKind::DirectCall)
        .expect("direct call");
    let end_index = instructions
        .iter()
        .position(|item| item.kind() == VerifiedInstructionKind::EndBorrow)
        .expect("end borrow");
    let post_end_index =
        instructions.iter().position(|item| item.kind() == post_end_kind).expect("post-end move");
    assert!(begin_index < call_index && call_index < end_index && end_index < post_end_index);
    let begin = instructions[begin_index];
    let call = instructions[call_index];
    let end = instructions[end_index];
    assert_eq!(begin.place_operands().next().expect("projection").index(), projection);
    assert_eq!(
        call.call_arguments().collect::<Vec<_>>(),
        [VerifiedCallArgument::Borrow(
            call.failure_ended_borrows().next().expect("failure-ended borrow"),
        )]
    );
    assert_eq!(call.failure_ended_borrows().map(BorrowIdentity::index).collect::<Vec<_>>(), [0]);
    assert_eq!(call.derived_drop_actions().count(), failure_drop_count);
    assert_eq!(
        call.cleanup()
            .and_then(|id| function.cleanup_plans().find(|plan| plan.id() == id))
            .expect("CallTrap cleanup")
            .site()
            .role(),
        VerifiedCleanupRole::CallTrap
    );
    assert_eq!(end.borrow(), begin.borrow());
}

#[test]
fn static_struct_and_array_projection_calls_preserve_lexical_authority() {
    let (sources, linear, linux, struct_ty, array_ty, _) = mixed_aggregate_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    for (shape, aggregate) in
        [(StaticShape::Struct, struct_ty), (StaticShape::FixedArray, array_ty)]
    {
        for access in [raw::BorrowAccess::Shared, raw::BorrowAccess::Exclusive] {
            let verified = verify(
                static_call_program(&sources, &linear, &linux, aggregate, shape, access),
                &sources,
                entry,
                linear.clone(),
                linux.clone(),
            )
            .expect("static projection borrowed call");
            assert_call_scope(
                verified.modules().next().expect("module").functions().next().expect("caller"),
                1,
                VerifiedInstructionKind::MoveFromPlace,
                1,
            );
        }
    }
}

pub(super) fn enum_call_program(seed: &Seed, mode: Mode) -> raw::Program {
    let mut raw = seed.program();
    let caller = &mut raw.modules[0].functions[0];
    let span = caller.span;
    if mode == Mode::Exclusive {
        caller.cleanup_plans[0].actions.push(raw::DropAction::DropPlace(raw::PlaceId(1)));
    }
    let mut call_cleanup = caller.cleanup_plans[0].clone();
    call_cleanup.id = raw::CleanupPlanId(1);
    caller.cleanup_plans.push(call_cleanup);
    caller.places.extend([
        raw::Place {
            id: raw::PlaceId(5),
            ty: seed.root,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(4)),
        },
        raw::Place { id: raw::PlaceId(6), ty: seed.root, span, kind: raw::PlaceKind::Local(0) },
        raw::Place {
            id: raw::PlaceId(7),
            ty: seed.root,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(5)),
        },
        raw::Place { id: raw::PlaceId(8), ty: seed.root, span, kind: raw::PlaceKind::Local(1) },
    ]);
    let mut instructions =
        vec![caller.blocks[0].instructions[0].clone(), begin_borrow(0, 4, mode.access(), span)];
    instructions.extend([
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(4), ty: seed.root, span }),
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
                place: raw::PlaceId(6),
                value: raw::ValueId(4),
            },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(6) },
        },
        end_borrow(0, span),
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(5), ty: seed.root, span }),
            span,
            kind: raw::InstructionKind::GenericMoveFromPlace { place: raw::PlaceId(4) },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::InitializePlace {
                place: raw::PlaceId(8),
                value: raw::ValueId(5),
            },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(8) },
        },
    ]);
    caller.blocks[0].instructions = instructions;
    raw.modules[0].functions.push(clone_callee(span, seed.root, mode.access()));
    raw
}

#[test]
fn refined_enum_payload_calls_preserve_lexical_authority() {
    for mode in [Mode::Shared, Mode::Exclusive] {
        let seed = Seed::new(mode);
        let verified =
            seed.check(enum_call_program(&seed, mode)).expect("enum payload borrowed call");
        assert_call_scope(
            verified.modules().next().expect("module").functions().next().expect("caller"),
            4,
            VerifiedInstructionKind::GenericMoveFromPlace,
            2,
        );
    }
}

#[test]
fn projection_calls_reject_wrong_region_and_repeated_exclusive_then_recover() {
    let (sources, linear, linux, struct_ty, _, _) = mixed_aggregate_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let valid = static_call_program(
        &sources,
        &linear,
        &linux,
        struct_ty,
        StaticShape::Struct,
        raw::BorrowAccess::Exclusive,
    );
    let mut wrong_region = valid.clone();
    let raw::InstructionKind::BeginBorrow(definition) =
        &mut wrong_region.modules[0].functions[0].blocks[0].instructions[0].kind
    else {
        panic!("begin borrow")
    };
    definition.place = raw::PlaceId(2);

    let mut repeated = valid.clone();
    let callee_span = repeated.modules[0].functions[1].span;
    repeated.modules[0].functions[1].borrow_parameters.push(raw::BorrowParameter {
        id: raw::BorrowId(1),
        referent: raw::TypeId(2),
        access: raw::BorrowAccess::Exclusive,
        span: callee_span,
    });
    let raw::InstructionKind::DirectCall { arguments, .. } =
        &mut repeated.modules[0].functions[0].blocks[0].instructions[1].kind
    else {
        panic!("direct call")
    };
    arguments.push(raw::CallArgument::Borrow(raw::BorrowId(0)));

    for hostile in [wrong_region, repeated] {
        let reject = || {
            diagnostic_trace(
                verify(hostile.clone(), &sources, entry, linear.clone(), linux.clone())
                    .expect_err("hostile projection borrowed call"),
            )
        };
        let first = reject();
        assert_eq!(first, reject());
        assert!(first.iter().any(|(code, _, _)| code == "ZRYNA-I3011"), "{first:?}");
    }
    verify(valid, &sources, entry, linear, linux).expect("valid recovery");
}

#[test]
fn enum_payload_calls_reject_wrong_region_and_repeated_exclusive_then_recover() {
    let seed = Seed::new(Mode::Exclusive);
    let valid = enum_call_program(&seed, Mode::Exclusive);
    let mut wrong_region = valid.clone();
    let raw::InstructionKind::BeginBorrow(definition) =
        &mut wrong_region.modules[0].functions[0].blocks[0].instructions[1].kind
    else {
        panic!("begin borrow")
    };
    definition.place = raw::PlaceId(5);

    let mut repeated = valid.clone();
    let callee_span = repeated.modules[0].functions[1].span;
    repeated.modules[0].functions[1].borrow_parameters.push(raw::BorrowParameter {
        id: raw::BorrowId(1),
        referent: seed.root,
        access: raw::BorrowAccess::Exclusive,
        span: callee_span,
    });
    let raw::InstructionKind::DirectCall { arguments, .. } =
        &mut repeated.modules[0].functions[0].blocks[0].instructions[2].kind
    else {
        panic!("direct call")
    };
    arguments.push(raw::CallArgument::Borrow(raw::BorrowId(0)));

    for hostile in [wrong_region, repeated] {
        let reject = || {
            diagnostic_trace(seed.check(hostile.clone()).expect_err("hostile enum payload call"))
        };
        let first = reject();
        assert_eq!(first, reject());
        assert!(first.iter().any(|(code, _, _)| code == "ZRYNA-I3011"), "{first:?}");
    }
    seed.check(valid).expect("valid enum payload recovery");
}
