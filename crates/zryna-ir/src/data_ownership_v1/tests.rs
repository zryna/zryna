use zryna_diagnostics::PrimaryLocation;
use zryna_layout::{StorageTarget, raw as raw_layout};
use zryna_source::{SourceFileInput, SourceMap};

use std::collections::BTreeSet;

use super::{
    Errors, MAX_ACTIVE_BORROWS_PER_FUNCTION, MAX_AGGREGATE_OPERANDS, MAX_BLOCK_PARAMETERS,
    MAX_BLOCKS_PER_FUNCTION, MAX_CALL_EDGES, MAX_CFG_EDGES_PER_FUNCTION,
    MAX_CLEANUP_PLANS_PER_FUNCTION, MAX_DIAGNOSTICS, MAX_DROP_ACTIONS_PER_FUNCTION,
    MAX_FUNCTIONS_PER_MODULE, MAX_LOOP_NESTING, MAX_MODULES, MAX_NOMINAL_DECLARATIONS,
    MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION, MAX_PARAMETERS_PER_FUNCTION, MAX_PLACES_PER_FUNCTION,
    MAX_VALUES_PER_FUNCTION, RuntimeContractIdentity, raw, verify, verify_reducible_loops,
};

fn authorities() -> (SourceMap, zryna_layout::VerifiedLayouts, zryna_layout::VerifiedLayouts) {
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "main.zry".into(),
        text: "export function id(value: i32): i32 { return value; }".into(),
    }])
    .expect("source map");
    let file = sources.verify_file_id(0).expect("source file");
    let graph = raw_layout::Graph {
        modules: vec![raw_layout::Module {
            id: raw_layout::ModuleId(0),
            source_file: file,
            data_declarations: 2,
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
                kind: raw_layout::TypeKind::Struct {
                    module: raw_layout::ModuleId(0),
                    declaration: 0,
                    fields: vec![raw_layout::Field { ordinal: 0, ty: raw_layout::NodeId(2) }],
                },
            },
            raw_layout::TypeNode {
                id: raw_layout::NodeId(4),
                span: Some(sources.span(file, 7, 13).expect("nominal span")),
                kind: raw_layout::TypeKind::Enum {
                    module: raw_layout::ModuleId(0),
                    declaration: 1,
                    variants: vec![raw_layout::Variant {
                        ordinal: 0,
                        payload: Some(raw_layout::NodeId(2)),
                    }],
                },
            },
        ],
        program_roots: vec![],
    };
    let linear =
        zryna_layout::verify(&graph, &sources, StorageTarget::Linear32V1).expect("linear layouts");
    let linux = zryna_layout::verify(&graph, &sources, StorageTarget::LinuxX8664V1)
        .expect("native layouts");
    (sources, linear, linux)
}

fn program(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
) -> raw::Program {
    let file = sources.verify_file_id(0).expect("source file");
    let span = sources.span(file, 0, 53).expect("whole source span");
    raw::Program {
        authorities: raw::AuthorityClaims {
            runtime: RuntimeContractIdentity::OwnershipRuntimeV1,
            type_universe: linear.universe_identity().as_bytes(),
            linear32_fingerprint: *linear.fingerprint(),
            linux_x86_64_fingerprint: *linux.fingerprint(),
        },
        entry_module: raw::ModuleId(0),
        modules: vec![raw::Module {
            id: raw::ModuleId(0),
            source_file: file,
            data_declarations: 2,
            functions: vec![raw::Function {
                id: raw::FunctionId { module: raw::ModuleId(0), declaration: 0 },
                entry_export: Some("id".into()),
                span,
                parameters: vec![raw::ValueDefinition {
                    id: raw::ValueId(0),
                    ty: raw::TypeId(1),
                    span,
                }],
                borrow_parameters: vec![],
                result: raw::TypeId(1),
                places: vec![],
                blocks: vec![raw::Block {
                    id: raw::BlockId(0),
                    parameters: vec![],
                    instructions: vec![],
                    terminators: vec![raw::SpannedTerminator {
                        span,
                        kind: raw::Terminator::Return {
                            value: raw::ValueId(0),
                            cleanup: raw::CleanupPlanId(0),
                        },
                    }],
                }],
                cleanup_plans: vec![raw::CleanupPlan {
                    id: raw::CleanupPlanId(0),
                    span,
                    actions: vec![],
                }],
            }],
        }],
    }
}

#[test]
fn scalar_seed_binds_both_layouts_and_opaque_views() {
    let (sources, linear, linux) = authorities();
    let raw = program(&sources, &linear, &linux);
    let entry = sources.verify_file_id(0).expect("entry");
    let verified = verify(raw, &sources, entry, linear, linux).expect("verified M3 seed");
    assert_eq!(verified.modules().len(), 1);
    let function = verified.modules().next().expect("module").functions().next().expect("function");
    assert!(function.public_export().is_some());
    assert_eq!(function.blocks().next().expect("block").id().index(), 0);
    assert_eq!(verified.runtime_contract(), RuntimeContractIdentity::OwnershipRuntimeV1);
}

#[test]
fn forged_layout_fingerprint_fails_closed() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    raw.authorities.linear32_fingerprint[0] ^= 0xff;
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("forgery must fail");
    assert_eq!(diagnostics[0].code(), "ZRYNA-I3003");
}

#[test]
fn noncanonical_function_identity_is_rejected() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    raw.modules[0].functions[0].id.declaration = 1;
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("identity must fail");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3002"));
}

#[test]
fn module_limit_plus_one_is_terminal() {
    let (sources, linear, linux) = authorities();
    let seed = program(&sources, &linear, &linux);
    let mut raw = seed;
    raw.modules = (0..=MAX_MODULES)
        .map(|index| raw::Module {
            id: raw::ModuleId(u32::try_from(index).expect("test index fits u32")),
            source_file: sources.verify_file_id(0).expect("source"),
            data_declarations: 0,
            functions: vec![],
        })
        .collect();
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("limit must fail");
    assert_eq!(diagnostics[0].code(), "ZRYNA-I3201");
}

#[test]
fn unreachable_block_is_rejected() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let mut block = raw.modules[0].functions[0].blocks[0].clone();
    block.id = raw::BlockId(1);
    raw.modules[0].functions[0].blocks.push(block);
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("CFG must fail");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3007"));
}

#[test]
fn self_use_does_not_satisfy_dominance() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    raw.modules[0].functions[0].blocks[0].instructions.push(raw::Instruction {
        result: Some(raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span }),
        span,
        kind: raw::InstructionKind::I32Neg { operand: raw::ValueId(1) },
    });
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("dominance must fail");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3008"));
}

#[test]
fn exact_operation_types_are_rechecked() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    raw.modules[0].functions[0].blocks[0].instructions.push(raw::Instruction {
        result: Some(raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(0), span }),
        span,
        kind: raw::InstructionKind::I32Literal(1),
    });
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("types must fail");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3005"));
}

#[test]
fn move_from_uninitialized_place_is_rejected() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.places.push(raw::Place {
        id: raw::PlaceId(0),
        ty: raw::TypeId(2),
        span,
        kind: raw::PlaceKind::Local(0),
    });
    function.places.push(raw::Place {
        id: raw::PlaceId(1),
        ty: raw::TypeId(2),
        span,
        kind: raw::PlaceKind::Temporary(raw::ValueId(1)),
    });
    function.cleanup_plans[0].actions.push(raw::DropAction::DropPlace(raw::PlaceId(1)));
    function.blocks[0].instructions.push(raw::Instruction {
        result: Some(raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(2), span }),
        span,
        kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(0) },
    });
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("move must fail");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"));
}

#[test]
fn live_owned_parameter_requires_complete_cleanup() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.entry_export = None;
    function
        .parameters
        .insert(0, raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(2), span });
    function.parameters[1].id = raw::ValueId(1);
    if let raw::Terminator::Return { value, .. } = &mut function.blocks[0].terminators[0].kind {
        *value = raw::ValueId(1);
    }
    function.places.push(raw::Place {
        id: raw::PlaceId(0),
        ty: raw::TypeId(2),
        span,
        kind: raw::PlaceKind::Parameter(0),
    });
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("cleanup must fail");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3012"));
}

#[test]
fn cyclic_projection_fails_before_ownership_traversal() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.places.push(raw::Place {
        id: raw::PlaceId(0),
        ty: raw::TypeId(2),
        span,
        kind: raw::PlaceKind::StructField { base: raw::PlaceId(0), ordinal: 0 },
    });
    function.blocks[0].instructions.push(raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::InitializePlace {
            place: raw::PlaceId(0),
            value: raw::ValueId(0),
        },
    });
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("cycle must fail");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3006"));
}

#[test]
fn direct_call_cycle_is_rejected() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.blocks[0].instructions.push(raw::Instruction {
        result: Some(raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span }),
        span,
        kind: raw::InstructionKind::DirectCall {
            callee: function.id,
            arguments: vec![raw::CallArgument::Value(raw::ValueId(0))],
            cleanup: None,
        },
    });
    if let raw::Terminator::Return { value, .. } = &mut function.blocks[0].terminators[0].kind {
        *value = raw::ValueId(1);
    }
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("cycle must fail");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3009"));
}

#[test]
fn orphan_owned_value_is_rejected() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.entry_export = None;
    function.parameters.push(raw::ValueDefinition {
        id: raw::ValueId(1),
        ty: raw::TypeId(2),
        span,
    });
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("orphan must fail");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3008"));
}

#[test]
fn dense_borrow_begin_and_end_is_accepted() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.entry_export = None;
    function
        .parameters
        .insert(0, raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(2), span });
    function.parameters[1].id = raw::ValueId(1);
    function.places.push(raw::Place {
        id: raw::PlaceId(0),
        ty: raw::TypeId(2),
        span,
        kind: raw::PlaceKind::Parameter(0),
    });
    function.cleanup_plans[0].actions.push(raw::DropAction::DropPlace(raw::PlaceId(0)));
    function.blocks[0].instructions.extend([
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::BeginBorrow(raw::BorrowDefinition {
                id: raw::BorrowId(0),
                place: raw::PlaceId(0),
                access: raw::BorrowAccess::Shared,
                span,
            }),
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::EndBorrow { borrow: raw::BorrowId(0) },
        },
    ]);
    if let raw::Terminator::Return { value, .. } = &mut function.blocks[0].terminators[0].kind {
        *value = raw::ValueId(1);
    }
    let entry = sources.verify_file_id(0).expect("entry");
    verify(raw, &sources, entry, linear, linux).expect("borrow-balanced program");
}

#[test]
fn orphan_cleanup_plan_is_rejected() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    raw.modules[0].functions[0].cleanup_plans.push(raw::CleanupPlan {
        id: raw::CleanupPlanId(1),
        span,
        actions: vec![],
    });
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("orphan plan");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3012"));
}

#[test]
fn verified_views_expose_only_owner_branded_operands_and_edges() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.blocks[0].instructions.push(raw::Instruction {
        result: Some(raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span }),
        span,
        kind: raw::InstructionKind::I32Neg { operand: raw::ValueId(0) },
    });
    if let raw::Terminator::Return { value, .. } = &mut function.blocks[0].terminators[0].kind {
        *value = raw::ValueId(1);
    }
    let entry = sources.verify_file_id(0).expect("entry");
    let verified = verify(raw, &sources, entry, linear, linux).expect("verified");
    let block = verified
        .modules()
        .next()
        .expect("verified module")
        .functions()
        .next()
        .expect("verified function")
        .blocks()
        .next()
        .expect("verified block");
    let instruction = block.instructions().next().expect("verified instruction");
    assert_eq!(instruction.value_operands().next().expect("value operand").index(), 0);
    assert_eq!(instruction.result_type().expect("result type").index(), 1);
    assert_eq!(block.terminator().value_operands().next().expect("return operand").index(), 1);
    assert_eq!(block.terminator().cleanup().expect("return cleanup").index(), 0);
}

#[test]
fn dominance_diagnostic_is_source_located() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    raw.modules[0].functions[0].blocks[0].instructions.push(raw::Instruction {
        result: Some(raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span }),
        span,
        kind: raw::InstructionKind::I32Neg { operand: raw::ValueId(1) },
    });
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("dominance");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code() == "ZRYNA-I3008")
        .expect("dominance diagnostic");
    assert!(matches!(diagnostic.primary(), PrimaryLocation::Source { .. }));
}

#[test]
fn irreducible_two_entry_cycle_is_rejected() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.parameters.push(raw::ValueDefinition {
        id: raw::ValueId(1),
        ty: raw::TypeId(0),
        span,
    });
    let edge = |target| raw::Edge { target: raw::BlockId(target), arguments: vec![] };
    function.blocks = vec![
        raw::Block {
            id: raw::BlockId(0),
            parameters: vec![],
            instructions: vec![],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Branch {
                    condition: raw::ValueId(1),
                    when_true: edge(1),
                    when_false: edge(2),
                },
            }],
        },
        raw::Block {
            id: raw::BlockId(1),
            parameters: vec![],
            instructions: vec![],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Jump(edge(2)),
            }],
        },
        raw::Block {
            id: raw::BlockId(2),
            parameters: vec![],
            instructions: vec![],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Branch {
                    condition: raw::ValueId(1),
                    when_true: edge(1),
                    when_false: edge(3),
                },
            }],
        },
        raw::Block {
            id: raw::BlockId(3),
            parameters: vec![],
            instructions: vec![],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Return {
                    value: raw::ValueId(0),
                    cleanup: raw::CleanupPlanId(0),
                },
            }],
        },
    ];
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("irreducible");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3007"));
}

#[test]
fn loop_nesting_limit_plus_one_is_terminal() {
    let repeated = MAX_LOOP_NESTING + 1;
    let successors = vec![vec![0; repeated]];
    let predecessors = vec![vec![0; repeated]];
    let dominators = vec![BTreeSet::from([0])];
    let mut errors = Errors::default();
    verify_reducible_loops(&successors, &predecessors, &dominators, None, &mut errors);
    assert_eq!(errors.finish()[0].code(), "ZRYNA-I3201");
}

#[test]
fn enum_payload_use_requires_matching_arm_dominance() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(4), span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span },
    ];
    function.places = vec![
        raw::Place {
            id: raw::PlaceId(0),
            ty: raw::TypeId(4),
            span,
            kind: raw::PlaceKind::Parameter(0),
        },
        raw::Place {
            id: raw::PlaceId(1),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::EnumPayload { base: raw::PlaceId(0), variant: 0 },
        },
    ];
    function.blocks[0].instructions.push(raw::Instruction {
        result: Some(raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(2), span }),
        span,
        kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(1) },
    });
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("payload dominance");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3013"));
}

#[test]
fn partial_move_cleanup_keeps_parent_obligation() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(3), span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span },
    ];
    function.places = vec![
        raw::Place {
            id: raw::PlaceId(0),
            ty: raw::TypeId(3),
            span,
            kind: raw::PlaceKind::Parameter(0),
        },
        raw::Place {
            id: raw::PlaceId(1),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(0), ordinal: 0 },
        },
        raw::Place {
            id: raw::PlaceId(2),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(2)),
        },
    ];
    function.blocks[0].instructions.push(raw::Instruction {
        result: Some(raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(2), span }),
        span,
        kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(1) },
    });
    if let raw::Terminator::Return { value, .. } = &mut function.blocks[0].terminators[0].kind {
        *value = raw::ValueId(1);
    }
    function.cleanup_plans[0].actions = vec![
        raw::DropAction::DropPlace(raw::PlaceId(2)),
        raw::DropAction::DropPlace(raw::PlaceId(0)),
    ];
    let entry = sources.verify_file_id(0).expect("entry");
    verify(raw, &sources, entry, linear, linux).expect("partial cleanup remains complete");
}

#[test]
fn active_enum_payload_move_has_exact_cleanup_order() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(4), span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span },
    ];
    function.places = vec![
        raw::Place {
            id: raw::PlaceId(0),
            ty: raw::TypeId(4),
            span,
            kind: raw::PlaceKind::Parameter(0),
        },
        raw::Place {
            id: raw::PlaceId(1),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::EnumPayload { base: raw::PlaceId(0), variant: 0 },
        },
        raw::Place {
            id: raw::PlaceId(2),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(2)),
        },
    ];
    function.blocks = vec![
        raw::Block {
            id: raw::BlockId(0),
            parameters: vec![],
            instructions: vec![],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::EnumMatch {
                    place: raw::PlaceId(0),
                    arms: vec![raw::EnumArm {
                        variant: 0,
                        edge: raw::Edge { target: raw::BlockId(1), arguments: vec![] },
                    }],
                },
            }],
        },
        raw::Block {
            id: raw::BlockId(1),
            parameters: vec![],
            instructions: vec![raw::Instruction {
                result: Some(raw::ValueDefinition {
                    id: raw::ValueId(2),
                    ty: raw::TypeId(2),
                    span,
                }),
                span,
                kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(1) },
            }],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Return {
                    value: raw::ValueId(1),
                    cleanup: raw::CleanupPlanId(0),
                },
            }],
        },
    ];
    function.cleanup_plans[0].actions = vec![
        raw::DropAction::DropPlace(raw::PlaceId(2)),
        raw::DropAction::DropPlace(raw::PlaceId(0)),
    ];
    let entry = sources.verify_file_id(0).expect("entry");
    let verified =
        verify(raw, &sources, entry, linear, linux).expect("active enum cleanup is exact");
    let function = verified.modules().next().expect("module").functions().next().expect("function");
    assert_eq!(function.parameters().len(), 2);
    assert_eq!(function.result_type().index(), 1);
    assert!(matches!(
        function.places().next().expect("place").kind(),
        super::VerifiedPlaceKind::Parameter(0)
    ));
    let arm =
        function.blocks().next().expect("entry").terminator().enum_arms().next().expect("enum arm");
    assert_eq!(arm.variant(), 0);
    assert_eq!(arm.edge().target().index(), 1);
    let cleanup = function
        .blocks()
        .nth(1)
        .expect("variant block")
        .terminator()
        .derived_drop_actions()
        .collect::<Vec<_>>();
    assert_eq!(cleanup.len(), 2);
    assert_eq!(cleanup[1].root().index(), 0);
    assert_eq!(cleanup[1].active_variant(), Some(0));
    assert_eq!(
        cleanup[1].moved_projections().map(super::PlaceIdentity::index).collect::<Vec<_>>(),
        [1]
    );
}

#[test]
fn verified_views_retain_literal_and_trap_payloads() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.blocks[0].instructions.push(raw::Instruction {
        result: Some(raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span }),
        span,
        kind: raw::InstructionKind::I32Literal(37),
    });
    function.blocks[0].terminators[0].kind = raw::Terminator::Trap {
        identity: raw::TrapIdentity::BoundsV1,
        cleanup: raw::CleanupPlanId(0),
    };
    let entry = sources.verify_file_id(0).expect("entry");
    let verified = verify(raw, &sources, entry, linear, linux).expect("sealed views");
    let block = verified
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function")
        .blocks()
        .next()
        .expect("block");
    assert_eq!(block.parameters().len(), 0);
    assert_eq!(block.instructions().next().expect("literal").i32_literal(), Some(37));
    assert_eq!(block.terminator().trap_identity(), Some(super::VerifiedTrapIdentity::BoundsV1));
    assert_eq!(block.terminator().cleanup().expect("cleanup").index(), 0);
}

#[test]
fn borrow_parameter_is_an_authenticated_active_authority() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.entry_export = None;
    function.borrow_parameters = vec![raw::BorrowParameter {
        id: raw::BorrowId(0),
        referent: raw::TypeId(1),
        access: raw::BorrowAccess::Shared,
        span,
    }];
    function.blocks[0].instructions.push(raw::Instruction {
        result: Some(raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span }),
        span,
        kind: raw::InstructionKind::BorrowRead { borrow: raw::BorrowId(0) },
    });
    if let raw::Terminator::Return { value, .. } = &mut function.blocks[0].terminators[0].kind {
        *value = raw::ValueId(1);
    }
    let entry = sources.verify_file_id(0).expect("entry");
    verify(raw, &sources, entry, linear, linux).expect("borrow parameter is active on entry");
}

#[test]
fn borrow_read_and_write_reject_non_copy_string_referents() {
    for write in [false, true] {
        let (sources, linear, linux) = authorities();
        let mut raw = program(&sources, &linear, &linux);
        let span = raw.modules[0].functions[0].span;
        let function = &mut raw.modules[0].functions[0];
        function.entry_export = None;
        function.borrow_parameters = vec![raw::BorrowParameter {
            id: raw::BorrowId(0),
            referent: raw::TypeId(2),
            access: if write { raw::BorrowAccess::Exclusive } else { raw::BorrowAccess::Shared },
            span,
        }];
        if write {
            function.parameters = vec![
                raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(2), span },
                raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span },
            ];
            function.blocks[0].instructions.push(raw::Instruction {
                result: None,
                span,
                kind: raw::InstructionKind::BorrowWrite {
                    borrow: raw::BorrowId(0),
                    value: raw::ValueId(0),
                },
            });
            function.blocks[0].terminators[0].kind =
                raw::Terminator::Return { value: raw::ValueId(1), cleanup: raw::CleanupPlanId(0) };
        } else {
            function.result = raw::TypeId(2);
            function.blocks[0].instructions.push(raw::Instruction {
                result: Some(raw::ValueDefinition {
                    id: raw::ValueId(1),
                    ty: raw::TypeId(2),
                    span,
                }),
                span,
                kind: raw::InstructionKind::BorrowRead { borrow: raw::BorrowId(0) },
            });
            function.blocks[0].terminators[0].kind =
                raw::Terminator::Return { value: raw::ValueId(1), cleanup: raw::CleanupPlanId(0) };
        }
        let entry = sources.verify_file_id(0).expect("entry");
        let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("non-Copy borrow");
        assert!(diagnostics.iter().any(|item| item.code() == "ZRYNA-I3005"));
    }
}

#[test]
fn direct_call_rejects_borrow_authority_with_wrong_access() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    let caller = &mut raw.modules[0].functions[0];
    caller.entry_export = None;
    caller.places = vec![raw::Place {
        id: raw::PlaceId(0),
        ty: raw::TypeId(1),
        span,
        kind: raw::PlaceKind::Parameter(0),
    }];
    caller.blocks[0].instructions = vec![
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::BeginBorrow(raw::BorrowDefinition {
                id: raw::BorrowId(0),
                place: raw::PlaceId(0),
                access: raw::BorrowAccess::Shared,
                span,
            }),
        },
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span }),
            span,
            kind: raw::InstructionKind::DirectCall {
                callee: raw::FunctionId { module: raw::ModuleId(0), declaration: 1 },
                arguments: vec![
                    raw::CallArgument::Value(raw::ValueId(0)),
                    raw::CallArgument::Borrow(raw::BorrowId(0)),
                ],
                cleanup: None,
            },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::EndBorrow { borrow: raw::BorrowId(0) },
        },
    ];
    if let raw::Terminator::Return { value, .. } = &mut caller.blocks[0].terminators[0].kind {
        *value = raw::ValueId(1);
    }
    raw.modules[0].functions.push(raw::Function {
        id: raw::FunctionId { module: raw::ModuleId(0), declaration: 1 },
        entry_export: None,
        span,
        parameters: vec![raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(1), span }],
        borrow_parameters: vec![raw::BorrowParameter {
            id: raw::BorrowId(0),
            referent: raw::TypeId(1),
            access: raw::BorrowAccess::Exclusive,
            span,
        }],
        result: raw::TypeId(1),
        places: vec![],
        blocks: vec![raw::Block {
            id: raw::BlockId(0),
            parameters: vec![],
            instructions: vec![],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Return {
                    value: raw::ValueId(0),
                    cleanup: raw::CleanupPlanId(0),
                },
            }],
        }],
        cleanup_plans: vec![raw::CleanupPlan { id: raw::CleanupPlanId(0), span, actions: vec![] }],
    });
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("access mismatch");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3009"));
}

#[test]
fn direct_call_rejects_repeated_exclusive_borrow_authority() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    let caller = &mut raw.modules[0].functions[0];
    caller.entry_export = None;
    caller.places = vec![raw::Place {
        id: raw::PlaceId(0),
        ty: raw::TypeId(1),
        span,
        kind: raw::PlaceKind::Parameter(0),
    }];
    caller.blocks[0].instructions = vec![
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::BeginBorrow(raw::BorrowDefinition {
                id: raw::BorrowId(0),
                place: raw::PlaceId(0),
                access: raw::BorrowAccess::Exclusive,
                span,
            }),
        },
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span }),
            span,
            kind: raw::InstructionKind::DirectCall {
                callee: raw::FunctionId { module: raw::ModuleId(0), declaration: 1 },
                arguments: vec![
                    raw::CallArgument::Value(raw::ValueId(0)),
                    raw::CallArgument::Borrow(raw::BorrowId(0)),
                    raw::CallArgument::Borrow(raw::BorrowId(0)),
                ],
                cleanup: None,
            },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::EndBorrow { borrow: raw::BorrowId(0) },
        },
    ];
    caller.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(1), cleanup: raw::CleanupPlanId(0) };
    raw.modules[0].functions.push(raw::Function {
        id: raw::FunctionId { module: raw::ModuleId(0), declaration: 1 },
        entry_export: None,
        span,
        parameters: vec![raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(1), span }],
        borrow_parameters: vec![
            raw::BorrowParameter {
                id: raw::BorrowId(0),
                referent: raw::TypeId(1),
                access: raw::BorrowAccess::Exclusive,
                span,
            },
            raw::BorrowParameter {
                id: raw::BorrowId(1),
                referent: raw::TypeId(1),
                access: raw::BorrowAccess::Exclusive,
                span,
            },
        ],
        result: raw::TypeId(1),
        places: vec![],
        blocks: vec![raw::Block {
            id: raw::BlockId(0),
            parameters: vec![],
            instructions: vec![],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Return {
                    value: raw::ValueId(0),
                    cleanup: raw::CleanupPlanId(0),
                },
            }],
        }],
        cleanup_plans: vec![raw::CleanupPlan { id: raw::CleanupPlanId(0), span, actions: vec![] }],
    });
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("duplicate authority");
    assert!(diagnostics.iter().any(|item| item.code() == "ZRYNA-I3009"));
}

fn preflight_codes(program: &raw::Program, layouts: &zryna_layout::VerifiedLayouts) -> Vec<String> {
    let mut errors = Errors::default();
    super::preflight(program, layouts, &mut errors);
    errors.finish().into_iter().map(|diagnostic| diagnostic.code).collect()
}

#[test]
fn m3_nominal_budget_accepts_exact_and_rejects_first_extra() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    raw.modules[0].data_declarations =
        u32::try_from(MAX_NOMINAL_DECLARATIONS).expect("nominal limit fits u32");
    assert!(preflight_codes(&raw, &linear).is_empty());
    raw.modules[0].data_declarations += 1;
    assert_eq!(preflight_codes(&raw, &linear), ["ZRYNA-I3201"]);
}

#[test]
fn m3_place_and_borrow_budgets_accept_exact_and_reject_first_extra() {
    let (sources, linear, linux) = authorities();
    let seed = program(&sources, &linear, &linux);
    let span = seed.modules[0].functions[0].span;
    let place = raw::Place {
        id: raw::PlaceId(0),
        ty: raw::TypeId(1),
        span,
        kind: raw::PlaceKind::Local(0),
    };
    let mut exact = seed.clone();
    exact.modules[0].functions[0].places = vec![place.clone(); MAX_PLACES_PER_FUNCTION];
    assert!(preflight_codes(&exact, &linear).is_empty());
    exact.modules[0].functions[0].places.push(place);
    assert_eq!(preflight_codes(&exact, &linear), ["ZRYNA-I3201"]);

    let borrow = raw::BorrowParameter {
        id: raw::BorrowId(0),
        referent: raw::TypeId(1),
        access: raw::BorrowAccess::Shared,
        span,
    };
    let mut exact = seed;
    exact.modules[0].functions[0].borrow_parameters = vec![borrow; MAX_ACTIVE_BORROWS_PER_FUNCTION];
    assert!(preflight_codes(&exact, &linear).is_empty());
    exact.modules[0].functions[0].borrow_parameters.push(borrow);
    assert_eq!(preflight_codes(&exact, &linear), ["ZRYNA-I3201"]);
    let _ = linux;
}

#[test]
fn m3_transition_and_aggregate_budgets_accept_exact_and_reject_first_extra() {
    let (sources, linear, linux) = authorities();
    let seed = program(&sources, &linear, &linux);
    let span = seed.modules[0].functions[0].span;
    let instruction = raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(0) },
    };
    let mut exact = seed.clone();
    exact.modules[0].functions[0].blocks[0].instructions =
        vec![instruction.clone(); MAX_OWNERSHIP_TRANSITIONS_PER_FUNCTION];
    assert!(preflight_codes(&exact, &linear).is_empty());
    exact.modules[0].functions[0].blocks[0].instructions.push(instruction);
    assert_eq!(preflight_codes(&exact, &linear), ["ZRYNA-I3201"]);

    let mut exact = seed;
    exact.modules[0].functions[0].blocks[0].instructions.push(raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::StructConstruct {
            fields: vec![raw::ValueId(0); MAX_AGGREGATE_OPERANDS],
            cleanup: None,
        },
    });
    assert!(preflight_codes(&exact, &linear).is_empty());
    if let raw::InstructionKind::StructConstruct { fields, .. } =
        &mut exact.modules[0].functions[0].blocks[0].instructions[0].kind
    {
        fields.push(raw::ValueId(0));
    }
    assert_eq!(preflight_codes(&exact, &linear), ["ZRYNA-I3201"]);
    let _ = linux;
}

#[test]
fn m3_drop_budget_accepts_exact_and_rejects_first_extra() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    raw.modules[0].functions[0].cleanup_plans[0].actions =
        vec![raw::DropAction::DropPlace(raw::PlaceId(0)); MAX_DROP_ACTIONS_PER_FUNCTION];
    assert!(preflight_codes(&raw, &linear).is_empty());
    raw.modules[0].functions[0].cleanup_plans[0]
        .actions
        .push(raw::DropAction::DropPlace(raw::PlaceId(0)));
    assert_eq!(preflight_codes(&raw, &linear), ["ZRYNA-I3201"]);
    let _ = linux;
}

#[test]
fn diagnostic_budget_retains_terminal_diagnostic_at_exact_capacity() {
    let mut errors = Errors::default();
    for _ in 0..MAX_DIAGNOSTICS {
        errors.push(super::error("ZRYNA-I3001", "fixture", "fixture"));
    }
    let diagnostics = errors.finish();
    assert_eq!(diagnostics.len(), MAX_DIAGNOSTICS);
    assert_eq!(diagnostics.last().expect("terminal diagnostic").code(), "ZRYNA-I3202");
}

#[test]
fn duplicate_non_copy_consumption_on_one_reachable_path_is_rejected() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(2), span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span },
    ];
    function.places = vec![
        raw::Place {
            id: raw::PlaceId(0),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Local(0),
        },
        raw::Place {
            id: raw::PlaceId(1),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Local(1),
        },
    ];
    function.blocks[0].instructions = vec![
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::InitializePlace {
                place: raw::PlaceId(0),
                value: raw::ValueId(0),
            },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::InitializePlace {
                place: raw::PlaceId(1),
                value: raw::ValueId(0),
            },
        },
    ];
    if let raw::Terminator::Return { value, .. } = &mut function.blocks[0].terminators[0].kind {
        *value = raw::ValueId(1);
    }
    function.cleanup_plans[0].actions = vec![
        raw::DropAction::DropPlace(raw::PlaceId(1)),
        raw::DropAction::DropPlace(raw::PlaceId(0)),
    ];
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("double consume");
    let diagnostic =
        diagnostics.iter().find(|item| item.code() == "ZRYNA-I3008").expect("owner diagnostic");
    assert!(matches!(diagnostic.primary(), PrimaryLocation::Source { .. }));
}

#[test]
fn partial_initialization_cleanup_exposes_exact_initialized_projection_set() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(2), span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span },
    ];
    function.places = vec![
        raw::Place {
            id: raw::PlaceId(0),
            ty: raw::TypeId(3),
            span,
            kind: raw::PlaceKind::Local(0),
        },
        raw::Place {
            id: raw::PlaceId(1),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(0), ordinal: 0 },
        },
    ];
    function.blocks[0].instructions = vec![raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::InitializePlace {
            place: raw::PlaceId(1),
            value: raw::ValueId(0),
        },
    }];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(1), cleanup: raw::CleanupPlanId(0) };
    function.cleanup_plans[0].actions = vec![raw::DropAction::DropPlace(raw::PlaceId(0))];
    let entry = sources.verify_file_id(0).expect("entry");
    let verified = verify(raw, &sources, entry, linear, linux).expect("partial initialization");
    let action = verified
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("function")
        .blocks()
        .next()
        .expect("block")
        .terminator()
        .derived_drop_actions()
        .next()
        .expect("drop action");
    assert_eq!(
        action.initialized_projections().map(super::PlaceIdentity::index).collect::<Vec<_>>(),
        [1]
    );
}

#[test]
fn duplicate_string_parameter_roots_are_rejected_before_cleanup() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(2), span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span },
    ];
    function.places = vec![
        raw::Place {
            id: raw::PlaceId(0),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Parameter(0),
        },
        raw::Place {
            id: raw::PlaceId(1),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Parameter(0),
        },
    ];
    if let raw::Terminator::Return { value, .. } = &mut function.blocks[0].terminators[0].kind {
        *value = raw::ValueId(1);
    }
    function.cleanup_plans[0].actions = vec![
        raw::DropAction::DropPlace(raw::PlaceId(1)),
        raw::DropAction::DropPlace(raw::PlaceId(0)),
    ];
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("root alias");
    assert!(diagnostics.iter().any(|item| item.code() == "ZRYNA-I3006"));
}

#[test]
fn duplicate_canonical_projection_is_rejected() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.entry_export = None;
    function.parameters[0].ty = raw::TypeId(3);
    function.places = vec![
        raw::Place {
            id: raw::PlaceId(0),
            ty: raw::TypeId(3),
            span,
            kind: raw::PlaceKind::Parameter(0),
        },
        raw::Place {
            id: raw::PlaceId(1),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(0), ordinal: 0 },
        },
        raw::Place {
            id: raw::PlaceId(2),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(0), ordinal: 0 },
        },
    ];
    function.cleanup_plans[0].actions = vec![raw::DropAction::DropPlace(raw::PlaceId(0))];
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("projection alias");
    assert!(diagnostics.iter().any(|item| item.code() == "ZRYNA-I3006"));
}

#[test]
fn eq_rejects_non_scalar_operands() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(2), span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(2), span },
    ];
    function.result = raw::TypeId(0);
    function.blocks[0].instructions.push(raw::Instruction {
        result: Some(raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(0), span }),
        span,
        kind: raw::InstructionKind::Eq { lhs: raw::ValueId(0), rhs: raw::ValueId(1) },
    });
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(2), cleanup: raw::CleanupPlanId(0) };
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("non-scalar eq");
    assert!(diagnostics.iter().any(|item| item.code() == "ZRYNA-I3005"));
}

#[test]
fn exclusive_borrow_blocks_owner_copy_read() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.entry_export = None;
    function.places = vec![raw::Place {
        id: raw::PlaceId(0),
        ty: raw::TypeId(1),
        span,
        kind: raw::PlaceKind::Parameter(0),
    }];
    function.blocks[0].instructions = vec![
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::BeginBorrow(raw::BorrowDefinition {
                id: raw::BorrowId(0),
                place: raw::PlaceId(0),
                access: raw::BorrowAccess::Exclusive,
                span,
            }),
        },
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span }),
            span,
            kind: raw::InstructionKind::CopyFromPlace { place: raw::PlaceId(0) },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::EndBorrow { borrow: raw::BorrowId(0) },
        },
    ];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(1), cleanup: raw::CleanupPlanId(0) };
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("owner read");
    assert!(diagnostics.iter().any(|item| item.code() == "ZRYNA-I3010"));
}

#[test]
fn empty_cleanup_plan_and_peak_borrow_budgets_have_exact_boundaries() {
    let (sources, linear, linux) = authorities();
    let seed = program(&sources, &linear, &linux);
    let span = seed.modules[0].functions[0].span;
    let plan = raw::CleanupPlan { id: raw::CleanupPlanId(0), span, actions: vec![] };
    let mut plans = seed.clone();
    plans.modules[0].functions[0].cleanup_plans =
        vec![plan.clone(); MAX_CLEANUP_PLANS_PER_FUNCTION];
    assert!(preflight_codes(&plans, &linear).is_empty());
    plans.modules[0].functions[0].cleanup_plans.push(plan);
    assert_eq!(preflight_codes(&plans, &linear), ["ZRYNA-I3201"]);

    let begin = raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::BeginBorrow(raw::BorrowDefinition {
            id: raw::BorrowId(0),
            place: raw::PlaceId(0),
            access: raw::BorrowAccess::Shared,
            span,
        }),
    };
    let end = raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::EndBorrow { borrow: raw::BorrowId(0) },
    };
    let mut borrows = seed;
    borrows.modules[0].functions[0].blocks[0].instructions =
        vec![begin.clone(); MAX_ACTIVE_BORROWS_PER_FUNCTION];
    assert!(preflight_codes(&borrows, &linear).is_empty());
    borrows.modules[0].functions[0].blocks[0].instructions.push(begin);
    assert_eq!(preflight_codes(&borrows, &linear), ["ZRYNA-I3201"]);
    borrows.modules[0].functions[0].blocks[0].instructions = (0..=MAX_ACTIVE_BORROWS_PER_FUNCTION)
        .flat_map(|_| {
            [
                end.clone(),
                raw::Instruction {
                    result: None,
                    span,
                    kind: raw::InstructionKind::BeginBorrow(raw::BorrowDefinition {
                        id: raw::BorrowId(0),
                        place: raw::PlaceId(0),
                        access: raw::BorrowAccess::Shared,
                        span,
                    }),
                },
            ]
        })
        .collect();
    assert!(preflight_codes(&borrows, &linear).is_empty());
    let _ = linux;
}

#[test]
fn inherited_m2_local_budgets_accept_exact_and_reject_first_extra() {
    let (sources, linear, linux) = authorities();
    let seed = program(&sources, &linear, &linux);
    let function = seed.modules[0].functions[0].clone();
    let mut functions = seed.clone();
    functions.modules[0].functions = vec![function; MAX_FUNCTIONS_PER_MODULE];
    assert!(preflight_codes(&functions, &linear).is_empty());
    let extra_function = functions.modules[0].functions[0].clone();
    functions.modules[0].functions.push(extra_function);
    assert_eq!(preflight_codes(&functions, &linear), ["ZRYNA-I3201"]);

    let parameter = seed.modules[0].functions[0].parameters[0];
    let mut parameters = seed.clone();
    parameters.modules[0].functions[0].parameters = vec![parameter; MAX_PARAMETERS_PER_FUNCTION];
    assert!(preflight_codes(&parameters, &linear).is_empty());
    parameters.modules[0].functions[0].parameters.push(parameter);
    assert_eq!(preflight_codes(&parameters, &linear), ["ZRYNA-I3201"]);

    let block = seed.modules[0].functions[0].blocks[0].clone();
    let mut blocks = seed.clone();
    blocks.modules[0].functions[0].blocks = vec![block; MAX_BLOCKS_PER_FUNCTION];
    assert!(preflight_codes(&blocks, &linear).is_empty());
    let extra_block = blocks.modules[0].functions[0].blocks[0].clone();
    blocks.modules[0].functions[0].blocks.push(extra_block);
    assert_eq!(preflight_codes(&blocks, &linear), ["ZRYNA-I3201"]);

    let mut block_parameters = seed.clone();
    block_parameters.modules[0].functions[0].blocks[0].parameters =
        vec![parameter; MAX_BLOCK_PARAMETERS];
    assert!(preflight_codes(&block_parameters, &linear).is_empty());
    block_parameters.modules[0].functions[0].blocks[0].parameters.push(parameter);
    assert_eq!(preflight_codes(&block_parameters, &linear), ["ZRYNA-I3201"]);

    let span = seed.modules[0].functions[0].span;
    let result = raw::Instruction {
        result: Some(parameter),
        span,
        kind: raw::InstructionKind::I32Literal(0),
    };
    let mut values = seed.clone();
    values.modules[0].functions[0].blocks[0].instructions =
        vec![result.clone(); MAX_VALUES_PER_FUNCTION - 1];
    assert!(preflight_codes(&values, &linear).is_empty());
    values.modules[0].functions[0].blocks[0].instructions.push(result);
    assert_eq!(preflight_codes(&values, &linear), ["ZRYNA-I3201"]);

    let edge =
        raw::EnumArm { variant: 0, edge: raw::Edge { target: raw::BlockId(0), arguments: vec![] } };
    let mut edges = seed.clone();
    edges.modules[0].functions[0].blocks[0].terminators[0].kind = raw::Terminator::EnumMatch {
        place: raw::PlaceId(0),
        arms: vec![edge.clone(); MAX_CFG_EDGES_PER_FUNCTION],
    };
    assert!(preflight_codes(&edges, &linear).is_empty());
    if let raw::Terminator::EnumMatch { arms, .. } =
        &mut edges.modules[0].functions[0].blocks[0].terminators[0].kind
    {
        arms.push(edge);
    }
    assert_eq!(preflight_codes(&edges, &linear), ["ZRYNA-I3201"]);

    let call = raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::DirectCall {
            callee: raw::FunctionId { module: raw::ModuleId(0), declaration: 0 },
            arguments: vec![],
            cleanup: None,
        },
    };
    let mut calls = seed;
    calls.modules[0].functions[0].blocks[0].instructions = vec![call.clone(); MAX_CALL_EDGES];
    assert!(preflight_codes(&calls, &linear).is_empty());
    calls.modules[0].functions[0].blocks[0].instructions.push(call);
    assert_eq!(preflight_codes(&calls, &linear), ["ZRYNA-I3201"]);
    let _ = linux;
}
