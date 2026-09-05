use super::*;

#[derive(Clone, Copy)]
enum Mutation {
    None,
    Write,
    Call,
}

fn authorities() -> (SourceMap, zryna_layout::VerifiedLayouts, zryna_layout::VerifiedLayouts) {
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "main.zry".into(),
        text: "export function id(value: i32): i32 { return value; }".into(),
    }])
    .expect("source authority");
    let file = sources.verify_file_id(0).expect("file");
    let graph = raw_layout::Graph {
        modules: vec![raw_layout::Module {
            id: raw_layout::ModuleId(0),
            source_file: file,
            data_declarations: 1,
        }],
        types: vec![
            raw_layout::TypeNode {
                id: raw_layout::NodeId(0),
                span: None,
                kind: raw_layout::TypeKind::Bool,
            },
            raw_layout::TypeNode {
                id: raw_layout::NodeId(1),
                span: None,
                kind: raw_layout::TypeKind::I32,
            },
            raw_layout::TypeNode {
                id: raw_layout::NodeId(2),
                span: None,
                kind: raw_layout::TypeKind::String,
            },
            raw_layout::TypeNode {
                id: raw_layout::NodeId(3),
                span: Some(sources.span(file, 0, 6).expect("nominal span")),
                kind: raw_layout::TypeKind::Enum {
                    module: raw_layout::ModuleId(0),
                    declaration: 0,
                    variants: vec![
                        raw_layout::Variant { ordinal: 0, payload: Some(raw_layout::NodeId(1)) },
                        raw_layout::Variant { ordinal: 1, payload: Some(raw_layout::NodeId(1)) },
                    ],
                },
            },
            raw_layout::TypeNode {
                id: raw_layout::NodeId(4),
                span: None,
                kind: raw_layout::TypeKind::FixedArray {
                    element: raw_layout::NodeId(3),
                    length: 1,
                },
            },
        ],
        program_roots: vec![raw_layout::NodeId(4)],
    };
    let linear = zryna_layout::verify(&graph, &sources, StorageTarget::Linear32V1)
        .expect("Copy enum array linear layout");
    let linux = zryna_layout::verify(&graph, &sources, StorageTarget::LinuxX8664V1)
        .expect("Copy enum array native layout");
    (sources, linear, linux)
}

fn fixture(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
    mutation: Mutation,
    read_old_payload: bool,
) -> raw::Program {
    let ty = |category| {
        raw::TypeId(linear.types().find(|ty| ty.category() == category).expect("type").id().index())
    };
    let integer = ty(zryna_layout::TypeCategory::I32);
    let element = ty(zryna_layout::TypeCategory::Enum);
    let array = ty(zryna_layout::TypeCategory::FixedArray);
    let mut raw = program(sources, linear, linux);
    raw.modules[0].data_declarations = 1;
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    let value = |id, ty| raw::ValueDefinition { id: raw::ValueId(id), ty, span };
    function.entry_export = None;
    function.parameters = vec![value(0, array), value(1, integer), value(2, integer)];
    function.result = integer;
    function.places = vec![
        raw::Place { id: raw::PlaceId(0), ty: array, span, kind: raw::PlaceKind::Parameter(0) },
        raw::Place {
            id: raw::PlaceId(1),
            ty: element,
            span,
            kind: raw::PlaceKind::FixedArrayConstant { base: raw::PlaceId(0), index: 0 },
        },
        raw::Place {
            id: raw::PlaceId(2),
            ty: integer,
            span,
            kind: raw::PlaceKind::EnumPayload { base: raw::PlaceId(1), variant: 0 },
        },
    ];
    let (instructions, next_value) =
        arm_instructions(span, integer, element, mutation, read_old_payload);
    let returns = |id| {
        vec![raw::SpannedTerminator {
            span,
            kind: raw::Terminator::Return {
                value: raw::ValueId(if id == 1 && read_old_payload { next_value } else { 2 }),
                cleanup: raw::CleanupPlanId(id),
            },
        }]
    };
    function.blocks = vec![
        raw::Block {
            id: raw::BlockId(0),
            parameters: vec![],
            instructions: vec![],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::EnumMatch {
                    place: raw::PlaceId(1),
                    arms: vec![
                        raw::EnumArm {
                            variant: 0,
                            edge: raw::Edge { target: raw::BlockId(1), arguments: vec![] },
                        },
                        raw::EnumArm {
                            variant: 1,
                            edge: raw::Edge { target: raw::BlockId(2), arguments: vec![] },
                        },
                    ],
                },
            }],
        },
        raw::Block {
            id: raw::BlockId(1),
            parameters: vec![],
            instructions,
            terminators: returns(1),
        },
        raw::Block {
            id: raw::BlockId(2),
            parameters: vec![],
            instructions: vec![],
            terminators: returns(2),
        },
    ];
    let plans = if matches!(mutation, Mutation::Call) { 4 } else { 3 };
    function.cleanup_plans = (0..plans)
        .map(|id| raw::CleanupPlan { id: raw::CleanupPlanId(id), span, actions: vec![] })
        .collect();
    if matches!(mutation, Mutation::Call) {
        let callee = mutation_callee(function, integer, element);
        raw.modules[0].functions.push(callee);
    }
    raw
}

fn arm_instructions(
    span: zryna_source::Span,
    integer: raw::TypeId,
    element: raw::TypeId,
    mutation: Mutation,
    read_old_payload: bool,
) -> (Vec<raw::Instruction>, u32) {
    let value = |id, ty| raw::ValueDefinition { id: raw::ValueId(id), ty, span };
    let mut instructions = vec![
        raw::Instruction {
            result: Some(value(3, element)),
            span,
            kind: raw::InstructionKind::EnumConstruct {
                variant: 1,
                payload: Some(raw::ValueId(2)),
                cleanup: None,
            },
        },
        indexed_borrow_fixture::indexed(0, 0, 1, 0, raw::BorrowAccess::Exclusive, span),
    ];
    let mut next_value = 4;
    match mutation {
        Mutation::None => {}
        Mutation::Write => instructions.push(raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::BorrowWrite {
                borrow: raw::BorrowId(0),
                value: raw::ValueId(3),
            },
        }),
        Mutation::Call => {
            instructions.push(raw::Instruction {
                result: Some(value(4, integer)),
                span,
                kind: raw::InstructionKind::DirectCall {
                    callee: raw::FunctionId { module: raw::ModuleId(0), declaration: 1 },
                    arguments: vec![
                        raw::CallArgument::Value(raw::ValueId(3)),
                        raw::CallArgument::Value(raw::ValueId(2)),
                        raw::CallArgument::Borrow(raw::BorrowId(0)),
                    ],
                    cleanup: raw::CleanupPlanId(3),
                },
            });
            next_value += 1;
        }
    }
    instructions.push(end_borrow(0, span));
    if read_old_payload {
        instructions.push(raw::Instruction {
            result: Some(value(next_value, integer)),
            span,
            kind: raw::InstructionKind::CopyFromPlace { place: raw::PlaceId(2) },
        });
    }
    (instructions, next_value)
}

fn mutation_callee(
    function: &raw::Function,
    integer: raw::TypeId,
    element: raw::TypeId,
) -> raw::Function {
    let span = function.span;
    let value = |id, ty| raw::ValueDefinition { id: raw::ValueId(id), ty, span };
    let mut callee = function.clone();
    callee.id.declaration = 1;
    callee.parameters = vec![value(0, element), value(1, integer)];
    callee.borrow_parameters = vec![raw::BorrowParameter {
        id: raw::BorrowId(0),
        referent: element,
        access: raw::BorrowAccess::Exclusive,
        span,
    }];
    callee.places.clear();
    callee.blocks = vec![raw::Block {
        id: raw::BlockId(0),
        parameters: vec![],
        instructions: vec![raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::BorrowWrite {
                borrow: raw::BorrowId(0),
                value: raw::ValueId(0),
            },
        }],
        terminators: vec![raw::SpannedTerminator {
            span,
            kind: raw::Terminator::Return {
                value: raw::ValueId(1),
                cleanup: raw::CleanupPlanId(0),
            },
        }],
    }];
    callee.cleanup_plans =
        vec![raw::CleanupPlan { id: raw::CleanupPlanId(0), span, actions: vec![] }];
    callee
}

#[test]
fn indexed_copy_enum_writes_and_exclusive_calls_invalidate_static_payload_refinement() {
    for mutation in [Mutation::None, Mutation::Write, Mutation::Call] {
        let (sources, linear, linux) = authorities();
        let raw = fixture(&sources, &linear, &linux, mutation, matches!(mutation, Mutation::None));
        let entry = sources.verify_file_id(0).expect("entry");
        let verified =
            verify(raw, &sources, entry, linear, linux).expect("complete accepted control");
        let function =
            verified.modules().next().expect("module").functions().next().expect("caller");
        let arm = function.blocks().nth(1).expect("refined arm");
        assert_eq!(arm.instructions().filter(|i| i.indexed_borrow().is_some()).count(), 1);
        if matches!(mutation, Mutation::Call) {
            let call = arm
                .instructions()
                .find(|i| i.kind() == VerifiedInstructionKind::DirectCall)
                .expect("exclusive mutation call");
            assert_eq!(
                call.failure_ended_borrows()
                    .map(super::super::BorrowIdentity::index)
                    .collect::<Vec<_>>(),
                vec![0]
            );
            assert_eq!(
                call.derived_drop_actions().count(),
                0,
                "Copy container never acquires cleanup"
            );
        }
        if matches!(mutation, Mutation::None) {
            continue;
        }
        for _ in 0..2 {
            let (sources, linear, linux) = authorities();
            let raw = fixture(&sources, &linear, &linux, mutation, true);
            let entry = sources.verify_file_id(0).expect("entry");
            let diagnostics =
                verify(raw, &sources, entry, linear, linux).expect_err("stale payload proof");
            assert!(
                diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3013"
                    && diagnostic.message()
                        == "enum payload ownership operation does not match the active variant"),
                "{diagnostics:?}"
            );
        }
    }
}
