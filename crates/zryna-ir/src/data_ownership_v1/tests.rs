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
    MAX_STATIC_CALL_DEPTH, MAX_STRING_LITERAL_BYTES, MAX_VALUES_PER_FUNCTION,
    RuntimeContractIdentity, VerifiedActiveVariant, VerifiedCleanupRole, VerifiedDropActionKind,
    VerifiedInstructionKind, VerifiedTerminatorKind, raw, verify, verify_reducible_loops,
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
            raw_layout::TypeNode {
                id: raw_layout::NodeId(5),
                span: None,
                kind: raw_layout::TypeKind::Vec { element: raw_layout::NodeId(2) },
            },
            raw_layout::TypeNode {
                id: raw_layout::NodeId(6),
                span: None,
                kind: raw_layout::TypeKind::Shared { payload: raw_layout::NodeId(2) },
            },
            raw_layout::TypeNode {
                id: raw_layout::NodeId(7),
                span: None,
                kind: raw_layout::TypeKind::Weak { payload: raw_layout::NodeId(2) },
            },
            raw_layout::TypeNode {
                id: raw_layout::NodeId(8),
                span: None,
                kind: raw_layout::TypeKind::Vec { element: raw_layout::NodeId(1) },
            },
        ],
        program_roots: vec![
            raw_layout::NodeId(5),
            raw_layout::NodeId(6),
            raw_layout::NodeId(7),
            raw_layout::NodeId(8),
        ],
    };
    let linear =
        zryna_layout::verify(&graph, &sources, StorageTarget::Linear32V1).expect("linear layouts");
    let linux = zryna_layout::verify(&graph, &sources, StorageTarget::LinuxX8664V1)
        .expect("native layouts");
    (sources, linear, linux)
}

fn payloadless_enum_authorities()
-> (SourceMap, zryna_layout::VerifiedLayouts, zryna_layout::VerifiedLayouts) {
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
                        raw_layout::Variant { ordinal: 0, payload: None },
                        raw_layout::Variant { ordinal: 1, payload: Some(raw_layout::NodeId(2)) },
                    ],
                },
            },
        ],
        program_roots: vec![raw_layout::NodeId(3)],
    };
    let linear =
        zryna_layout::verify(&graph, &sources, StorageTarget::Linear32V1).expect("linear layouts");
    let linux = zryna_layout::verify(&graph, &sources, StorageTarget::LinuxX8664V1)
        .expect("native layouts");
    (sources, linear, linux)
}

fn pair_authorities() -> (SourceMap, zryna_layout::VerifiedLayouts, zryna_layout::VerifiedLayouts) {
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
                kind: raw_layout::TypeKind::Struct {
                    module: raw_layout::ModuleId(0),
                    declaration: 0,
                    fields: vec![
                        raw_layout::Field { ordinal: 0, ty: raw_layout::NodeId(2) },
                        raw_layout::Field { ordinal: 1, ty: raw_layout::NodeId(2) },
                    ],
                },
            },
        ],
        program_roots: vec![raw_layout::NodeId(3)],
    };
    let linear =
        zryna_layout::verify(&graph, &sources, StorageTarget::Linear32V1).expect("linear layouts");
    let linux = zryna_layout::verify(&graph, &sources, StorageTarget::LinuxX8664V1)
        .expect("native layouts");
    (sources, linear, linux)
}

fn subobject_move_authorities() -> (
    SourceMap,
    zryna_layout::VerifiedLayouts,
    zryna_layout::VerifiedLayouts,
    raw::TypeId,
    raw::TypeId,
    raw::TypeId,
) {
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
                span: Some(sources.span(file, 0, 6).expect("inner span")),
                kind: raw_layout::TypeKind::Struct {
                    module: raw_layout::ModuleId(0),
                    declaration: 0,
                    fields: vec![raw_layout::Field { ordinal: 0, ty: raw_layout::NodeId(2) }],
                },
            },
            raw_layout::TypeNode {
                id: raw_layout::NodeId(4),
                span: Some(sources.span(file, 7, 13).expect("outer span")),
                kind: raw_layout::TypeKind::Struct {
                    module: raw_layout::ModuleId(0),
                    declaration: 1,
                    fields: vec![
                        raw_layout::Field { ordinal: 0, ty: raw_layout::NodeId(3) },
                        raw_layout::Field { ordinal: 1, ty: raw_layout::NodeId(2) },
                    ],
                },
            },
            raw_layout::TypeNode {
                id: raw_layout::NodeId(5),
                span: None,
                kind: raw_layout::TypeKind::FixedArray {
                    element: raw_layout::NodeId(3),
                    length: 2,
                },
            },
        ],
        program_roots: vec![raw_layout::NodeId(4), raw_layout::NodeId(5)],
    };
    let linear =
        zryna_layout::verify(&graph, &sources, StorageTarget::Linear32V1).expect("linear layouts");
    let linux = zryna_layout::verify(&graph, &sources, StorageTarget::LinuxX8664V1)
        .expect("native layouts");
    let inner = linear
        .types()
        .find(|ty| ty.nominal_identity() == Some((0, 0)))
        .map(|ty| raw::TypeId(ty.id().index()))
        .expect("inner type");
    let outer = linear
        .types()
        .find(|ty| ty.nominal_identity() == Some((0, 1)))
        .map(|ty| raw::TypeId(ty.id().index()))
        .expect("outer type");
    let array = linear
        .types()
        .find(|ty| {
            ty.category() == zryna_layout::TypeCategory::FixedArray
                && ty.array_length() == Some(2)
                && ty.referenced_type().is_some_and(|element| element.index() == inner.0)
        })
        .map(|ty| raw::TypeId(ty.id().index()))
        .expect("inner array type");
    (sources, linear, linux, inner, outer, array)
}

fn enum_payload_move_authorities() -> (
    SourceMap,
    zryna_layout::VerifiedLayouts,
    zryna_layout::VerifiedLayouts,
    raw::TypeId,
    raw::TypeId,
) {
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
                span: Some(sources.span(file, 0, 6).expect("payload span")),
                kind: raw_layout::TypeKind::Struct {
                    module: raw_layout::ModuleId(0),
                    declaration: 0,
                    fields: vec![raw_layout::Field { ordinal: 0, ty: raw_layout::NodeId(2) }],
                },
            },
            raw_layout::TypeNode {
                id: raw_layout::NodeId(4),
                span: Some(sources.span(file, 7, 13).expect("enum span")),
                kind: raw_layout::TypeKind::Enum {
                    module: raw_layout::ModuleId(0),
                    declaration: 1,
                    variants: vec![raw_layout::Variant {
                        ordinal: 0,
                        payload: Some(raw_layout::NodeId(3)),
                    }],
                },
            },
        ],
        program_roots: vec![raw_layout::NodeId(4)],
    };
    let linear =
        zryna_layout::verify(&graph, &sources, StorageTarget::Linear32V1).expect("linear layouts");
    let linux = zryna_layout::verify(&graph, &sources, StorageTarget::LinuxX8664V1)
        .expect("native layouts");
    let payload = linear
        .types()
        .find(|ty| ty.nominal_identity() == Some((0, 0)))
        .map(|ty| raw::TypeId(ty.id().index()))
        .expect("payload type");
    let enum_ty = linear
        .types()
        .find(|ty| ty.nominal_identity() == Some((0, 1)))
        .map(|ty| raw::TypeId(ty.id().index()))
        .expect("enum type");
    (sources, linear, linux, payload, enum_ty)
}

fn mixed_aggregate_authorities() -> (
    SourceMap,
    zryna_layout::VerifiedLayouts,
    zryna_layout::VerifiedLayouts,
    raw::TypeId,
    raw::TypeId,
    raw::TypeId,
) {
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
                    fields: vec![
                        raw_layout::Field { ordinal: 0, ty: raw_layout::NodeId(1) },
                        raw_layout::Field { ordinal: 1, ty: raw_layout::NodeId(2) },
                    ],
                },
            },
            raw_layout::TypeNode {
                id: raw_layout::NodeId(4),
                span: None,
                kind: raw_layout::TypeKind::FixedArray {
                    element: raw_layout::NodeId(2),
                    length: 2,
                },
            },
            raw_layout::TypeNode {
                id: raw_layout::NodeId(5),
                span: Some(sources.span(file, 7, 13).expect("enum span")),
                kind: raw_layout::TypeKind::Enum {
                    module: raw_layout::ModuleId(0),
                    declaration: 1,
                    variants: vec![
                        raw_layout::Variant { ordinal: 0, payload: Some(raw_layout::NodeId(3)) },
                        raw_layout::Variant { ordinal: 1, payload: Some(raw_layout::NodeId(3)) },
                    ],
                },
            },
        ],
        program_roots: vec![raw_layout::NodeId(3), raw_layout::NodeId(4), raw_layout::NodeId(5)],
    };
    let linear =
        zryna_layout::verify(&graph, &sources, StorageTarget::Linear32V1).expect("linear layouts");
    let linux = zryna_layout::verify(&graph, &sources, StorageTarget::LinuxX8664V1)
        .expect("native layouts");
    let struct_ty = linear
        .types()
        .find(|ty| ty.nominal_identity() == Some((0, 0)))
        .map(|ty| raw::TypeId(ty.id().index()))
        .expect("mixed struct");
    let enum_ty = linear
        .types()
        .find(|ty| ty.nominal_identity() == Some((0, 1)))
        .map(|ty| raw::TypeId(ty.id().index()))
        .expect("nested enum");
    let array_ty = linear
        .types()
        .find(|ty| {
            ty.category() == zryna_layout::TypeCategory::FixedArray
                && ty.array_length() == Some(2)
                && ty.referenced_type().is_some_and(|element| {
                    linear.type_by_id(element).is_some_and(|element| {
                        element.category() == zryna_layout::TypeCategory::String
                    })
                })
        })
        .map(|ty| raw::TypeId(ty.id().index()))
        .expect("String array");
    (sources, linear, linux, struct_ty, array_ty, enum_ty)
}

fn copy_clone_authorities()
-> (SourceMap, zryna_layout::VerifiedLayouts, zryna_layout::VerifiedLayouts, raw::TypeId) {
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
            data_declarations: 3,
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
                    fields: vec![raw_layout::Field { ordinal: 0, ty: raw_layout::NodeId(1) }],
                },
            },
            raw_layout::TypeNode {
                id: raw_layout::NodeId(4),
                span: None,
                kind: raw_layout::TypeKind::FixedArray {
                    element: raw_layout::NodeId(3),
                    length: 2,
                },
            },
            raw_layout::TypeNode {
                id: raw_layout::NodeId(5),
                span: Some(sources.span(file, 7, 13).expect("nominal span")),
                kind: raw_layout::TypeKind::Enum {
                    module: raw_layout::ModuleId(0),
                    declaration: 1,
                    variants: vec![raw_layout::Variant {
                        ordinal: 0,
                        payload: Some(raw_layout::NodeId(4)),
                    }],
                },
            },
            raw_layout::TypeNode {
                id: raw_layout::NodeId(6),
                span: Some(sources.span(file, 14, 20).expect("nominal span")),
                kind: raw_layout::TypeKind::Struct {
                    module: raw_layout::ModuleId(0),
                    declaration: 2,
                    fields: vec![raw_layout::Field { ordinal: 0, ty: raw_layout::NodeId(5) }],
                },
            },
        ],
        program_roots: vec![raw_layout::NodeId(6)],
    };
    let linear =
        zryna_layout::verify(&graph, &sources, StorageTarget::Linear32V1).expect("linear layouts");
    let linux = zryna_layout::verify(&graph, &sources, StorageTarget::LinuxX8664V1)
        .expect("native layouts");
    let ty = linear
        .types()
        .find(|ty| ty.nominal_identity() == Some((0, 2)))
        .map(|ty| raw::TypeId(ty.id().index()))
        .expect("outer Copy struct");
    (sources, linear, linux, ty)
}

fn runtime_child_clone_authorities(
    runtime_child: u8,
) -> (SourceMap, zryna_layout::VerifiedLayouts, zryna_layout::VerifiedLayouts, raw::TypeId) {
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "main.zry".into(),
        text: "export function id(value: i32): i32 { return value; }".into(),
    }])
    .expect("source map");
    let file = sources.verify_file_id(0).expect("source file");
    let kind = match runtime_child {
        0 => raw_layout::TypeKind::Vec { element: raw_layout::NodeId(2) },
        1 => raw_layout::TypeKind::Shared { payload: raw_layout::NodeId(2) },
        2 => raw_layout::TypeKind::Weak { payload: raw_layout::NodeId(2) },
        _ => panic!("runtime child selector"),
    };
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
            raw_layout::TypeNode { id: raw_layout::NodeId(3), span: None, kind },
            raw_layout::TypeNode {
                id: raw_layout::NodeId(4),
                span: Some(sources.span(file, 0, 6).expect("nominal span")),
                kind: raw_layout::TypeKind::Struct {
                    module: raw_layout::ModuleId(0),
                    declaration: 0,
                    fields: vec![raw_layout::Field { ordinal: 0, ty: raw_layout::NodeId(3) }],
                },
            },
        ],
        program_roots: vec![raw_layout::NodeId(4)],
    };
    let linear =
        zryna_layout::verify(&graph, &sources, StorageTarget::Linear32V1).expect("linear layouts");
    let linux = zryna_layout::verify(&graph, &sources, StorageTarget::LinuxX8664V1)
        .expect("native layouts");
    let ty = linear
        .types()
        .find(|ty| ty.nominal_identity() == Some((0, 0)))
        .map(|ty| raw::TypeId(ty.id().index()))
        .expect("runtime-owning struct");
    (sources, linear, linux, ty)
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

fn shared_borrow_read_program(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
) -> raw::Program {
    let mut raw = program(sources, linear, linux);
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.entry_export = None;
    function.places = vec![raw::Place {
        id: raw::PlaceId(0),
        ty: raw::TypeId(1),
        span,
        kind: raw::PlaceKind::Local(0),
    }];
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
            kind: raw::InstructionKind::BorrowRead { borrow: raw::BorrowId(0) },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::EndBorrow { borrow: raw::BorrowId(0) },
        },
    ];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(1), cleanup: raw::CleanupPlanId(0) };
    raw
}

#[derive(Clone, Copy)]
enum SubobjectMoveShape {
    Struct,
    FixedArray,
}

fn aggregate_subobject_move_program(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
    inner: raw::TypeId,
    outer: raw::TypeId,
    array: raw::TypeId,
    shape: SubobjectMoveShape,
) -> raw::Program {
    let mut raw = program(sources, linear, linux);
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    let root_ty = match shape {
        SubobjectMoveShape::Struct => outer,
        SubobjectMoveShape::FixedArray => array,
    };
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: root_ty, span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span },
    ];
    function.result = raw::TypeId(1);
    function.places = vec![
        raw::Place { id: raw::PlaceId(0), ty: root_ty, span, kind: raw::PlaceKind::Parameter(0) },
        raw::Place {
            id: raw::PlaceId(1),
            ty: inner,
            span,
            kind: match shape {
                SubobjectMoveShape::Struct => {
                    raw::PlaceKind::StructField { base: raw::PlaceId(0), ordinal: 0 }
                }
                SubobjectMoveShape::FixedArray => {
                    raw::PlaceKind::FixedArrayConstant { base: raw::PlaceId(0), index: 0 }
                }
            },
        },
        raw::Place {
            id: raw::PlaceId(2),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(1), ordinal: 0 },
        },
        raw::Place {
            id: raw::PlaceId(3),
            ty: inner,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(2)),
        },
        raw::Place {
            id: raw::PlaceId(4),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(3), ordinal: 0 },
        },
        raw::Place { id: raw::PlaceId(5), ty: inner, span, kind: raw::PlaceKind::Local(0) },
        raw::Place {
            id: raw::PlaceId(6),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(5), ordinal: 0 },
        },
    ];
    function.blocks[0].instructions = vec![
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(2), ty: inner, span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(1) },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::InitializePlace {
                place: raw::PlaceId(5),
                value: raw::ValueId(2),
            },
        },
    ];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(1), cleanup: raw::CleanupPlanId(0) };
    function.cleanup_plans[0].actions = vec![
        raw::DropAction::DropPlace(raw::PlaceId(5)),
        raw::DropAction::DropPlace(raw::PlaceId(0)),
    ];
    raw
}

#[allow(clippy::too_many_lines)]
fn aggregate_subobject_return_program(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
    inner: raw::TypeId,
    outer: raw::TypeId,
    array: raw::TypeId,
    shape: SubobjectMoveShape,
) -> raw::Program {
    let mut raw = program(sources, linear, linux);
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    let (source, returned, places, instructions, prepare_plans) = match shape {
        SubobjectMoveShape::Struct => (
            raw::PlaceId(5),
            raw::ValueId(4),
            vec![
                raw::Place {
                    id: raw::PlaceId(0),
                    ty: raw::TypeId(2),
                    span,
                    kind: raw::PlaceKind::Temporary(raw::ValueId(0)),
                },
                raw::Place {
                    id: raw::PlaceId(1),
                    ty: raw::TypeId(2),
                    span,
                    kind: raw::PlaceKind::Temporary(raw::ValueId(1)),
                },
                raw::Place {
                    id: raw::PlaceId(2),
                    ty: inner,
                    span,
                    kind: raw::PlaceKind::Temporary(raw::ValueId(2)),
                },
                raw::Place {
                    id: raw::PlaceId(3),
                    ty: outer,
                    span,
                    kind: raw::PlaceKind::Temporary(raw::ValueId(3)),
                },
                raw::Place { id: raw::PlaceId(4), ty: outer, span, kind: raw::PlaceKind::Local(0) },
                raw::Place {
                    id: raw::PlaceId(5),
                    ty: inner,
                    span,
                    kind: raw::PlaceKind::StructField { base: raw::PlaceId(4), ordinal: 0 },
                },
                raw::Place {
                    id: raw::PlaceId(6),
                    ty: raw::TypeId(2),
                    span,
                    kind: raw::PlaceKind::StructField { base: raw::PlaceId(5), ordinal: 0 },
                },
                raw::Place {
                    id: raw::PlaceId(7),
                    ty: inner,
                    span,
                    kind: raw::PlaceKind::Temporary(raw::ValueId(4)),
                },
            ],
            vec![
                raw::Instruction {
                    result: Some(raw::ValueDefinition {
                        id: raw::ValueId(0),
                        ty: raw::TypeId(2),
                        span,
                    }),
                    span,
                    kind: raw::InstructionKind::StringFromUtf8 {
                        bytes: b"tail".to_vec(),
                        cleanup: raw::CleanupPlanId(0),
                    },
                },
                raw::Instruction {
                    result: Some(raw::ValueDefinition {
                        id: raw::ValueId(1),
                        ty: raw::TypeId(2),
                        span,
                    }),
                    span,
                    kind: raw::InstructionKind::StringFromUtf8 {
                        bytes: b"inner".to_vec(),
                        cleanup: raw::CleanupPlanId(1),
                    },
                },
                raw::Instruction {
                    result: Some(raw::ValueDefinition { id: raw::ValueId(2), ty: inner, span }),
                    span,
                    kind: raw::InstructionKind::StructConstruct {
                        fields: vec![raw::ValueId(1)],
                        cleanup: None,
                    },
                },
                raw::Instruction {
                    result: Some(raw::ValueDefinition { id: raw::ValueId(3), ty: outer, span }),
                    span,
                    kind: raw::InstructionKind::StructConstruct {
                        fields: vec![raw::ValueId(2), raw::ValueId(0)],
                        cleanup: None,
                    },
                },
                raw::Instruction {
                    result: None,
                    span,
                    kind: raw::InstructionKind::InitializePlace {
                        place: raw::PlaceId(4),
                        value: raw::ValueId(3),
                    },
                },
                raw::Instruction {
                    result: Some(raw::ValueDefinition { id: raw::ValueId(4), ty: inner, span }),
                    span,
                    kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(5) },
                },
            ],
            vec![vec![], vec![raw::DropAction::DropPlace(raw::PlaceId(0))]],
        ),
        SubobjectMoveShape::FixedArray => (
            raw::PlaceId(6),
            raw::ValueId(5),
            vec![
                raw::Place {
                    id: raw::PlaceId(0),
                    ty: raw::TypeId(2),
                    span,
                    kind: raw::PlaceKind::Temporary(raw::ValueId(0)),
                },
                raw::Place {
                    id: raw::PlaceId(1),
                    ty: inner,
                    span,
                    kind: raw::PlaceKind::Temporary(raw::ValueId(1)),
                },
                raw::Place {
                    id: raw::PlaceId(2),
                    ty: raw::TypeId(2),
                    span,
                    kind: raw::PlaceKind::Temporary(raw::ValueId(2)),
                },
                raw::Place {
                    id: raw::PlaceId(3),
                    ty: inner,
                    span,
                    kind: raw::PlaceKind::Temporary(raw::ValueId(3)),
                },
                raw::Place {
                    id: raw::PlaceId(4),
                    ty: array,
                    span,
                    kind: raw::PlaceKind::Temporary(raw::ValueId(4)),
                },
                raw::Place { id: raw::PlaceId(5), ty: array, span, kind: raw::PlaceKind::Local(0) },
                raw::Place {
                    id: raw::PlaceId(6),
                    ty: inner,
                    span,
                    kind: raw::PlaceKind::FixedArrayConstant { base: raw::PlaceId(5), index: 0 },
                },
                raw::Place {
                    id: raw::PlaceId(7),
                    ty: raw::TypeId(2),
                    span,
                    kind: raw::PlaceKind::StructField { base: raw::PlaceId(6), ordinal: 0 },
                },
                raw::Place {
                    id: raw::PlaceId(8),
                    ty: inner,
                    span,
                    kind: raw::PlaceKind::Temporary(raw::ValueId(5)),
                },
            ],
            vec![
                raw::Instruction {
                    result: Some(raw::ValueDefinition {
                        id: raw::ValueId(0),
                        ty: raw::TypeId(2),
                        span,
                    }),
                    span,
                    kind: raw::InstructionKind::StringFromUtf8 {
                        bytes: b"left".to_vec(),
                        cleanup: raw::CleanupPlanId(0),
                    },
                },
                raw::Instruction {
                    result: Some(raw::ValueDefinition { id: raw::ValueId(1), ty: inner, span }),
                    span,
                    kind: raw::InstructionKind::StructConstruct {
                        fields: vec![raw::ValueId(0)],
                        cleanup: None,
                    },
                },
                raw::Instruction {
                    result: Some(raw::ValueDefinition {
                        id: raw::ValueId(2),
                        ty: raw::TypeId(2),
                        span,
                    }),
                    span,
                    kind: raw::InstructionKind::StringFromUtf8 {
                        bytes: b"right".to_vec(),
                        cleanup: raw::CleanupPlanId(1),
                    },
                },
                raw::Instruction {
                    result: Some(raw::ValueDefinition { id: raw::ValueId(3), ty: inner, span }),
                    span,
                    kind: raw::InstructionKind::StructConstruct {
                        fields: vec![raw::ValueId(2)],
                        cleanup: None,
                    },
                },
                raw::Instruction {
                    result: Some(raw::ValueDefinition { id: raw::ValueId(4), ty: array, span }),
                    span,
                    kind: raw::InstructionKind::FixedArrayConstruct {
                        elements: vec![raw::ValueId(1), raw::ValueId(3)],
                        cleanup: None,
                    },
                },
                raw::Instruction {
                    result: None,
                    span,
                    kind: raw::InstructionKind::InitializePlace {
                        place: raw::PlaceId(5),
                        value: raw::ValueId(4),
                    },
                },
                raw::Instruction {
                    result: Some(raw::ValueDefinition { id: raw::ValueId(5), ty: inner, span }),
                    span,
                    kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(6) },
                },
            ],
            vec![vec![], vec![raw::DropAction::DropPlace(raw::PlaceId(1))]],
        ),
    };
    function.entry_export = None;
    function.parameters.clear();
    function.result = inner;
    function.places = places;
    function.blocks[0].instructions = instructions;
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: returned, cleanup: raw::CleanupPlanId(2) };
    function.cleanup_plans = prepare_plans
        .into_iter()
        .enumerate()
        .map(|(index, actions)| raw::CleanupPlan {
            id: raw::CleanupPlanId(u32::try_from(index).expect("plan id")),
            span,
            actions,
        })
        .chain(std::iter::once(raw::CleanupPlan {
            id: raw::CleanupPlanId(2),
            span,
            actions: vec![raw::DropAction::DropPlace(root_place_for_test(
                source,
                &function.places,
            ))],
        }))
        .collect();
    raw
}

fn root_place_for_test(mut place: raw::PlaceId, places: &[raw::Place]) -> raw::PlaceId {
    while let Some(base) = places.get(place.0 as usize).and_then(|place| match place.kind {
        raw::PlaceKind::StructField { base, .. }
        | raw::PlaceKind::EnumPayload { base, .. }
        | raw::PlaceKind::FixedArrayConstant { base, .. } => Some(base),
        _ => None,
    }) {
        place = base;
    }
    place
}

fn projected_aggregate_clone_program(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
    inner: raw::TypeId,
    outer: raw::TypeId,
    array: raw::TypeId,
    shape: SubobjectMoveShape,
) -> raw::Program {
    let mut raw = program(sources, linear, linux);
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    let root_ty = match shape {
        SubobjectMoveShape::Struct => outer,
        SubobjectMoveShape::FixedArray => array,
    };
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: root_ty, span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span },
    ];
    function.result = raw::TypeId(1);
    function.places = vec![
        raw::Place { id: raw::PlaceId(0), ty: root_ty, span, kind: raw::PlaceKind::Parameter(0) },
        raw::Place {
            id: raw::PlaceId(1),
            ty: inner,
            span,
            kind: match shape {
                SubobjectMoveShape::Struct => {
                    raw::PlaceKind::StructField { base: raw::PlaceId(0), ordinal: 0 }
                }
                SubobjectMoveShape::FixedArray => {
                    raw::PlaceKind::FixedArrayConstant { base: raw::PlaceId(0), index: 0 }
                }
            },
        },
        raw::Place {
            id: raw::PlaceId(2),
            ty: inner,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(2)),
        },
        raw::Place { id: raw::PlaceId(3), ty: inner, span, kind: raw::PlaceKind::Local(0) },
    ];
    function.blocks[0].instructions = vec![
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(2), ty: inner, span }),
            span,
            kind: raw::InstructionKind::ClonePlace {
                place: raw::PlaceId(1),
                cleanup: raw::CleanupPlanId(0),
                element_cleanup: Some(raw::CleanupPlanId(1)),
            },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::InitializePlace {
                place: raw::PlaceId(3),
                value: raw::ValueId(2),
            },
        },
    ];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(1), cleanup: raw::CleanupPlanId(2) };
    function.cleanup_plans = vec![
        raw::CleanupPlan {
            id: raw::CleanupPlanId(0),
            span,
            actions: vec![raw::DropAction::DropPlace(raw::PlaceId(0))],
        },
        raw::CleanupPlan {
            id: raw::CleanupPlanId(1),
            span,
            actions: vec![
                raw::DropAction::DropAggregateInitializedPrefix(raw::PlaceId(2)),
                raw::DropAction::DropPlace(raw::PlaceId(0)),
            ],
        },
        raw::CleanupPlan {
            id: raw::CleanupPlanId(2),
            span,
            actions: vec![
                raw::DropAction::DropPlace(raw::PlaceId(3)),
                raw::DropAction::DropPlace(raw::PlaceId(0)),
            ],
        },
    ];
    raw
}

fn projected_aggregate_assignment_program(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
    inner: raw::TypeId,
    outer: raw::TypeId,
    array: raw::TypeId,
    shape: SubobjectMoveShape,
) -> raw::Program {
    let mut raw = program(sources, linear, linux);
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    let root_ty = match shape {
        SubobjectMoveShape::Struct => outer,
        SubobjectMoveShape::FixedArray => array,
    };
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: root_ty, span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: inner, span },
        raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(1), span },
    ];
    function.result = raw::TypeId(1);
    function.places = vec![
        raw::Place { id: raw::PlaceId(0), ty: root_ty, span, kind: raw::PlaceKind::Parameter(0) },
        raw::Place { id: raw::PlaceId(1), ty: inner, span, kind: raw::PlaceKind::Parameter(1) },
        raw::Place { id: raw::PlaceId(2), ty: root_ty, span, kind: raw::PlaceKind::Local(0) },
        raw::Place {
            id: raw::PlaceId(3),
            ty: inner,
            span,
            kind: match shape {
                SubobjectMoveShape::Struct => {
                    raw::PlaceKind::StructField { base: raw::PlaceId(2), ordinal: 0 }
                }
                SubobjectMoveShape::FixedArray => {
                    raw::PlaceKind::FixedArrayConstant { base: raw::PlaceId(2), index: 0 }
                }
            },
        },
        raw::Place { id: raw::PlaceId(4), ty: inner, span, kind: raw::PlaceKind::Local(1) },
        raw::Place {
            id: raw::PlaceId(5),
            ty: inner,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(3)),
        },
    ];
    function.blocks[0].instructions = vec![
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::InitializePlace {
                place: raw::PlaceId(2),
                value: raw::ValueId(0),
            },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::InitializePlace {
                place: raw::PlaceId(4),
                value: raw::ValueId(1),
            },
        },
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(3), ty: inner, span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(4) },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::ReplacePlace {
                place: raw::PlaceId(3),
                value: raw::ValueId(3),
            },
        },
    ];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(2), cleanup: raw::CleanupPlanId(0) };
    function.cleanup_plans[0].actions = vec![raw::DropAction::DropPlace(raw::PlaceId(2))];
    raw
}

#[allow(clippy::too_many_lines)]
fn projected_subobject_assignment_program(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
    inner: raw::TypeId,
    outer: raw::TypeId,
    array: raw::TypeId,
    shape: SubobjectMoveShape,
) -> raw::Program {
    let mut raw = program(sources, linear, linux);
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    let root_ty = match shape {
        SubobjectMoveShape::Struct => outer,
        SubobjectMoveShape::FixedArray => array,
    };
    let projection = |base| match shape {
        SubobjectMoveShape::Struct => raw::PlaceKind::StructField { base, ordinal: 0 },
        SubobjectMoveShape::FixedArray => raw::PlaceKind::FixedArrayConstant { base, index: 0 },
    };
    function.entry_export = None;
    function.parameters.clear();
    function.borrow_parameters.clear();
    function.result = raw::TypeId(1);
    function.places.clear();
    function.blocks[0].instructions.clear();
    function.cleanup_plans.clear();

    let mut next_value = 0u32;
    let mut next_place = 0u32;
    let mut next_cleanup = 0u32;
    let mut roots = Vec::with_capacity(2);
    for (ordinal, labels) in [(0, *b"od"), (1, *b"ns")] {
        let inherited = roots.last().copied();
        let mut elements = Vec::with_capacity(2);
        let mut first_inner = None;
        for (index, byte) in labels.into_iter().enumerate() {
            let string_value = raw::ValueId(next_value);
            next_value += 1;
            let string_place = raw::PlaceId(next_place);
            next_place += 1;
            function.places.push(raw::Place {
                id: string_place,
                ty: raw::TypeId(2),
                span,
                kind: raw::PlaceKind::Temporary(string_value),
            });
            let cleanup = raw::CleanupPlanId(next_cleanup);
            next_cleanup += 1;
            let mut actions = Vec::new();
            if let Some(inner_place) = first_inner {
                actions.push(raw::DropAction::DropPlace(inner_place));
            }
            if let Some(root) = inherited {
                actions.push(raw::DropAction::DropPlace(root));
            }
            function.cleanup_plans.push(raw::CleanupPlan { id: cleanup, span, actions });
            function.blocks[0].instructions.push(raw::Instruction {
                result: Some(raw::ValueDefinition { id: string_value, ty: raw::TypeId(2), span }),
                span,
                kind: raw::InstructionKind::StringFromUtf8 { bytes: vec![byte], cleanup },
            });
            if index == 0 || matches!(shape, SubobjectMoveShape::FixedArray) {
                let inner_value = raw::ValueId(next_value);
                next_value += 1;
                let inner_place = raw::PlaceId(next_place);
                next_place += 1;
                function.places.push(raw::Place {
                    id: inner_place,
                    ty: inner,
                    span,
                    kind: raw::PlaceKind::Temporary(inner_value),
                });
                function.blocks[0].instructions.push(raw::Instruction {
                    result: Some(raw::ValueDefinition { id: inner_value, ty: inner, span }),
                    span,
                    kind: raw::InstructionKind::StructConstruct {
                        fields: vec![string_value],
                        cleanup: None,
                    },
                });
                first_inner.get_or_insert(inner_place);
                elements.push(inner_value);
            } else {
                elements.push(string_value);
            }
        }
        let root_value = raw::ValueId(next_value);
        next_value += 1;
        let root_temporary = raw::PlaceId(next_place);
        next_place += 1;
        function.places.push(raw::Place {
            id: root_temporary,
            ty: root_ty,
            span,
            kind: raw::PlaceKind::Temporary(root_value),
        });
        let construct = match shape {
            SubobjectMoveShape::Struct => {
                raw::InstructionKind::StructConstruct { fields: elements, cleanup: None }
            }
            SubobjectMoveShape::FixedArray => {
                raw::InstructionKind::FixedArrayConstruct { elements, cleanup: None }
            }
        };
        function.blocks[0].instructions.push(raw::Instruction {
            result: Some(raw::ValueDefinition { id: root_value, ty: root_ty, span }),
            span,
            kind: construct,
        });
        let root = raw::PlaceId(next_place);
        next_place += 1;
        function.places.push(raw::Place {
            id: root,
            ty: root_ty,
            span,
            kind: raw::PlaceKind::Local(ordinal),
        });
        function.blocks[0].instructions.push(raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::InitializePlace { place: root, value: root_value },
        });
        roots.push(root);
    }

    let target_root = roots[0];
    let source_root = roots[1];
    let target = raw::PlaceId(next_place);
    next_place += 1;
    function.places.push(raw::Place { id: target, ty: inner, span, kind: projection(target_root) });
    let source = raw::PlaceId(next_place);
    next_place += 1;
    function.places.push(raw::Place { id: source, ty: inner, span, kind: projection(source_root) });
    let source_leaf = raw::PlaceId(next_place);
    next_place += 1;
    function.places.push(raw::Place {
        id: source_leaf,
        ty: raw::TypeId(2),
        span,
        kind: raw::PlaceKind::StructField { base: source, ordinal: 0 },
    });
    let moved = raw::ValueId(next_value);
    next_value += 1;
    let moved_temporary = raw::PlaceId(next_place);
    function.places.push(raw::Place {
        id: moved_temporary,
        ty: inner,
        span,
        kind: raw::PlaceKind::Temporary(moved),
    });
    function.blocks[0].instructions.extend([
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: moved, ty: inner, span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: source },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::ReplacePlace { place: target, value: moved },
        },
    ]);
    let result = raw::ValueId(next_value);
    function.blocks[0].instructions.push(raw::Instruction {
        result: Some(raw::ValueDefinition { id: result, ty: raw::TypeId(1), span }),
        span,
        kind: raw::InstructionKind::I32Literal(0),
    });
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: result, cleanup: raw::CleanupPlanId(next_cleanup) };
    function.cleanup_plans.push(raw::CleanupPlan {
        id: raw::CleanupPlanId(next_cleanup),
        span,
        actions: vec![
            raw::DropAction::DropPlace(source_root),
            raw::DropAction::DropPlace(target_root),
        ],
    });
    raw
}

fn projected_subobject_clone_assignment_program(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
    inner: raw::TypeId,
    outer: raw::TypeId,
    array: raw::TypeId,
    shape: SubobjectMoveShape,
) -> raw::Program {
    let mut raw =
        projected_subobject_assignment_program(sources, linear, linux, inner, outer, array, shape);
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    let move_index = function.blocks[0]
        .instructions
        .iter()
        .position(|instruction| {
            matches!(instruction.kind, raw::InstructionKind::MoveFromPlace { .. })
        })
        .expect("projected move");
    let raw::InstructionKind::MoveFromPlace { place: source } =
        function.blocks[0].instructions[move_index].kind
    else {
        unreachable!()
    };
    let result = function.blocks[0].instructions[move_index].result.expect("clone result");
    let raw::InstructionKind::ReplacePlace { place: target, .. } =
        function.blocks[0].instructions[move_index + 1].kind
    else {
        unreachable!()
    };
    let source_root =
        super::projection_base(&function.places[source.0 as usize].kind).expect("source root");
    let target_root =
        super::projection_base(&function.places[target.0 as usize].kind).expect("target root");
    let source_leaf = function
        .places
        .iter()
        .find(|place| super::projection_base(&place.kind) == Some(source))
        .expect("source leaf")
        .id;
    let result_temporary = function
        .places
        .iter()
        .find(|place| matches!(place.kind, raw::PlaceKind::Temporary(owner) if owner == result.id))
        .expect("clone temporary")
        .id;
    assert_eq!(result_temporary.0, source_leaf.0 + 1);
    assert_eq!(result_temporary.0 as usize + 1, function.places.len());
    function.places.remove(source_leaf.0 as usize);
    function.places[source_leaf.0 as usize].id = source_leaf;

    let cleanup = raw::CleanupPlanId(
        function.cleanup_plans.len().try_into().expect("bounded cleanup plan count"),
    );
    let element_cleanup = raw::CleanupPlanId(cleanup.0 + 1);
    function.cleanup_plans.extend([
        raw::CleanupPlan {
            id: cleanup,
            span,
            actions: vec![
                raw::DropAction::DropPlace(source_root),
                raw::DropAction::DropPlace(target_root),
            ],
        },
        raw::CleanupPlan {
            id: element_cleanup,
            span,
            actions: vec![
                raw::DropAction::DropAggregateInitializedPrefix(source_leaf),
                raw::DropAction::DropPlace(source_root),
                raw::DropAction::DropPlace(target_root),
            ],
        },
    ]);
    function.blocks[0].instructions[move_index].kind = raw::InstructionKind::ClonePlace {
        place: source,
        cleanup,
        element_cleanup: Some(element_cleanup),
    };
    raw
}

fn projected_aggregate_clone_assignment_program(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
    inner: raw::TypeId,
    outer: raw::TypeId,
    array: raw::TypeId,
    shape: SubobjectMoveShape,
) -> raw::Program {
    let mut raw =
        projected_aggregate_assignment_program(sources, linear, linux, inner, outer, array, shape);
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    function.blocks[0].instructions[2].kind = raw::InstructionKind::ClonePlace {
        place: raw::PlaceId(4),
        cleanup: raw::CleanupPlanId(1),
        element_cleanup: Some(raw::CleanupPlanId(2)),
    };
    function.cleanup_plans[0].actions = vec![
        raw::DropAction::DropPlace(raw::PlaceId(4)),
        raw::DropAction::DropPlace(raw::PlaceId(2)),
    ];
    function.cleanup_plans.extend([
        raw::CleanupPlan {
            id: raw::CleanupPlanId(1),
            span,
            actions: vec![
                raw::DropAction::DropPlace(raw::PlaceId(4)),
                raw::DropAction::DropPlace(raw::PlaceId(2)),
            ],
        },
        raw::CleanupPlan {
            id: raw::CleanupPlanId(2),
            span,
            actions: vec![
                raw::DropAction::DropAggregateInitializedPrefix(raw::PlaceId(5)),
                raw::DropAction::DropPlace(raw::PlaceId(4)),
                raw::DropAction::DropPlace(raw::PlaceId(2)),
            ],
        },
    ]);
    raw
}

#[allow(clippy::too_many_lines)]
fn enum_payload_move_program(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
    payload: raw::TypeId,
    enum_ty: raw::TypeId,
) -> raw::Program {
    let mut raw = program(sources, linear, linux);
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    function.entry_export = None;
    function.parameters = vec![raw::ValueDefinition { id: raw::ValueId(0), ty: enum_ty, span }];
    function.result = payload;
    function.places = vec![
        raw::Place { id: raw::PlaceId(0), ty: enum_ty, span, kind: raw::PlaceKind::Parameter(0) },
        raw::Place {
            id: raw::PlaceId(1),
            ty: payload,
            span,
            kind: raw::PlaceKind::EnumPayload { base: raw::PlaceId(0), variant: 0 },
        },
        raw::Place {
            id: raw::PlaceId(2),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(1), ordinal: 0 },
        },
        raw::Place {
            id: raw::PlaceId(3),
            ty: payload,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(1)),
        },
        raw::Place { id: raw::PlaceId(4), ty: payload, span, kind: raw::PlaceKind::Local(0) },
        raw::Place {
            id: raw::PlaceId(5),
            ty: payload,
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
            instructions: vec![
                raw::Instruction {
                    result: Some(raw::ValueDefinition { id: raw::ValueId(1), ty: payload, span }),
                    span,
                    kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(1) },
                },
                raw::Instruction {
                    result: None,
                    span,
                    kind: raw::InstructionKind::InitializePlace {
                        place: raw::PlaceId(4),
                        value: raw::ValueId(1),
                    },
                },
                raw::Instruction {
                    result: None,
                    span,
                    kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(0) },
                },
            ],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Jump(raw::Edge {
                    target: raw::BlockId(2),
                    arguments: vec![],
                }),
            }],
        },
        raw::Block {
            id: raw::BlockId(2),
            parameters: vec![],
            instructions: vec![raw::Instruction {
                result: Some(raw::ValueDefinition { id: raw::ValueId(2), ty: payload, span }),
                span,
                kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(4) },
            }],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Return {
                    value: raw::ValueId(2),
                    cleanup: raw::CleanupPlanId(0),
                },
            }],
        },
    ];
    function.cleanup_plans[0].actions.clear();
    raw
}

fn string_literal_program(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
    bytes: Vec<u8>,
) -> raw::Program {
    let mut program = program(sources, linear, linux);
    let function = &mut program.modules[0].functions[0];
    let span = function.span;
    function.entry_export = None;
    function.parameters.clear();
    function.result = raw::TypeId(2);
    function.places = vec![raw::Place {
        id: raw::PlaceId(0),
        ty: raw::TypeId(2),
        span,
        kind: raw::PlaceKind::Temporary(raw::ValueId(0)),
    }];
    function.blocks[0].instructions = vec![raw::Instruction {
        result: Some(raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(2), span }),
        span,
        kind: raw::InstructionKind::StringFromUtf8 { bytes, cleanup: raw::CleanupPlanId(0) },
    }];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(0), cleanup: raw::CleanupPlanId(1) };
    function.cleanup_plans.push(raw::CleanupPlan {
        id: raw::CleanupPlanId(1),
        span,
        actions: vec![],
    });
    program
}

fn string_clone_program(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
    ty: raw::TypeId,
    generic: bool,
) -> raw::Program {
    let mut program = program(sources, linear, linux);
    let function = &mut program.modules[0].functions[0];
    let span = function.span;
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty, span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span },
    ];
    function.result = raw::TypeId(1);
    function.places = vec![
        raw::Place { id: raw::PlaceId(0), ty, span, kind: raw::PlaceKind::Parameter(0) },
        raw::Place {
            id: raw::PlaceId(1),
            ty,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(2)),
        },
    ];
    let cleanup = raw::CleanupPlanId(0);
    function.blocks[0].instructions = vec![raw::Instruction {
        result: Some(raw::ValueDefinition { id: raw::ValueId(2), ty, span }),
        span,
        kind: if generic {
            raw::InstructionKind::ClonePlace {
                place: raw::PlaceId(0),
                cleanup,
                element_cleanup: None,
            }
        } else {
            raw::InstructionKind::StringClone { place: raw::PlaceId(0), cleanup }
        },
    }];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(1), cleanup: raw::CleanupPlanId(1) };
    function.cleanup_plans = vec![
        raw::CleanupPlan {
            id: cleanup,
            span,
            actions: vec![raw::DropAction::DropPlace(raw::PlaceId(0))],
        },
        raw::CleanupPlan {
            id: raw::CleanupPlanId(1),
            span,
            actions: vec![
                raw::DropAction::DropPlace(raw::PlaceId(1)),
                raw::DropAction::DropPlace(raw::PlaceId(0)),
            ],
        },
    ];
    program
}

fn string_bearing_aggregate_clone_program(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
    ty: raw::TypeId,
) -> raw::Program {
    let mut program = string_clone_program(sources, linear, linux, ty, true);
    let function = &mut program.modules[0].functions[0];
    let span = function.span;
    let element_cleanup = raw::CleanupPlanId(2);
    let raw::InstructionKind::ClonePlace { element_cleanup: claim, .. } =
        &mut function.blocks[0].instructions[0].kind
    else {
        panic!("ClonePlace")
    };
    *claim = Some(element_cleanup);
    function.cleanup_plans.push(raw::CleanupPlan {
        id: element_cleanup,
        span,
        actions: vec![
            raw::DropAction::DropAggregateInitializedPrefix(raw::PlaceId(1)),
            raw::DropAction::DropPlace(raw::PlaceId(0)),
        ],
    });
    program
}

fn payloadless_active_enum_clone_program(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
) -> raw::Program {
    let mut program = program(sources, linear, linux);
    program.modules[0].data_declarations = 1;
    let function = &mut program.modules[0].functions[0];
    let span = function.span;
    function.entry_export = None;
    function.parameters.clear();
    function.result = raw::TypeId(3);
    function.places = vec![
        raw::Place {
            id: raw::PlaceId(0),
            ty: raw::TypeId(3),
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(0)),
        },
        raw::Place {
            id: raw::PlaceId(1),
            ty: raw::TypeId(3),
            span,
            kind: raw::PlaceKind::Local(0),
        },
        raw::Place {
            id: raw::PlaceId(2),
            ty: raw::TypeId(3),
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(1)),
        },
    ];
    function.blocks[0].instructions = vec![
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(3), span }),
            span,
            kind: raw::InstructionKind::EnumConstruct { variant: 0, payload: None, cleanup: None },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::InitializePlace {
                place: raw::PlaceId(1),
                value: raw::ValueId(0),
            },
        },
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(3), span }),
            span,
            kind: raw::InstructionKind::ClonePlace {
                place: raw::PlaceId(1),
                cleanup: raw::CleanupPlanId(0),
                element_cleanup: Some(raw::CleanupPlanId(1)),
            },
        },
    ];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(1), cleanup: raw::CleanupPlanId(2) };
    function.cleanup_plans = vec![
        raw::CleanupPlan {
            id: raw::CleanupPlanId(0),
            span,
            actions: vec![raw::DropAction::DropPlace(raw::PlaceId(1))],
        },
        raw::CleanupPlan {
            id: raw::CleanupPlanId(1),
            span,
            actions: vec![
                raw::DropAction::DropAggregateInitializedPrefix(raw::PlaceId(2)),
                raw::DropAction::DropPlace(raw::PlaceId(1)),
            ],
        },
        raw::CleanupPlan {
            id: raw::CleanupPlanId(2),
            span,
            actions: vec![raw::DropAction::DropPlace(raw::PlaceId(1))],
        },
    ];
    program
}

fn owned_construct_program(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
    ty: raw::TypeId,
    enum_variant: bool,
) -> raw::Program {
    let mut program = program(sources, linear, linux);
    let function = &mut program.modules[0].functions[0];
    let span = function.span;
    function.entry_export = None;
    function.parameters =
        vec![raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(2), span }];
    function.result = ty;
    function.places = vec![
        raw::Place {
            id: raw::PlaceId(0),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Parameter(0),
        },
        raw::Place {
            id: raw::PlaceId(1),
            ty,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(1)),
        },
    ];
    function.blocks[0].instructions = vec![raw::Instruction {
        result: Some(raw::ValueDefinition { id: raw::ValueId(1), ty, span }),
        span,
        kind: if enum_variant {
            raw::InstructionKind::EnumConstruct {
                variant: 0,
                payload: Some(raw::ValueId(0)),
                cleanup: None,
            }
        } else {
            raw::InstructionKind::StructConstruct { fields: vec![raw::ValueId(0)], cleanup: None }
        },
    }];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(1), cleanup: raw::CleanupPlanId(0) };
    function.cleanup_plans[0].actions.clear();
    program
}

fn string_concat_program(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
    same_place: bool,
) -> raw::Program {
    let mut program = program(sources, linear, linux);
    let function = &mut program.modules[0].functions[0];
    let span = function.span;
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(2), span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(2), span },
        raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(1), span },
    ];
    function.result = raw::TypeId(1);
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
            kind: raw::PlaceKind::Parameter(1),
        },
        raw::Place {
            id: raw::PlaceId(2),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(3)),
        },
    ];
    function.blocks[0].instructions = vec![raw::Instruction {
        result: Some(raw::ValueDefinition { id: raw::ValueId(3), ty: raw::TypeId(2), span }),
        span,
        kind: raw::InstructionKind::StringConcat {
            left: raw::PlaceId(0),
            right: if same_place { raw::PlaceId(0) } else { raw::PlaceId(1) },
            cleanup: raw::CleanupPlanId(0),
        },
    }];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(2), cleanup: raw::CleanupPlanId(1) };
    function.cleanup_plans = vec![
        raw::CleanupPlan {
            id: raw::CleanupPlanId(0),
            span,
            actions: vec![
                raw::DropAction::DropPlace(raw::PlaceId(1)),
                raw::DropAction::DropPlace(raw::PlaceId(0)),
            ],
        },
        raw::CleanupPlan {
            id: raw::CleanupPlanId(1),
            span,
            actions: vec![
                raw::DropAction::DropPlace(raw::PlaceId(2)),
                raw::DropAction::DropPlace(raw::PlaceId(1)),
                raw::DropAction::DropPlace(raw::PlaceId(0)),
            ],
        },
    ];
    program
}

fn vec_string_construct_program(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
) -> raw::Program {
    let mut program = program(sources, linear, linux);
    let function = &mut program.modules[0].functions[0];
    let span = function.span;
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(2), span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(2), span },
        raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(1), span },
    ];
    function.result = raw::TypeId(1);
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
            kind: raw::PlaceKind::Parameter(1),
        },
        raw::Place {
            id: raw::PlaceId(2),
            ty: raw::TypeId(6),
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(3)),
        },
    ];
    function.blocks[0].instructions = vec![raw::Instruction {
        result: Some(raw::ValueDefinition { id: raw::ValueId(3), ty: raw::TypeId(6), span }),
        span,
        kind: raw::InstructionKind::VecConstruct {
            elements: vec![raw::ValueId(0), raw::ValueId(1)],
            cleanup: raw::CleanupPlanId(0),
        },
    }];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(2), cleanup: raw::CleanupPlanId(1) };
    function.cleanup_plans = vec![
        raw::CleanupPlan {
            id: raw::CleanupPlanId(0),
            span,
            actions: vec![
                raw::DropAction::DropPlace(raw::PlaceId(1)),
                raw::DropAction::DropPlace(raw::PlaceId(0)),
            ],
        },
        raw::CleanupPlan {
            id: raw::CleanupPlanId(1),
            span,
            actions: vec![raw::DropAction::DropPlace(raw::PlaceId(2))],
        },
    ];
    program
}

fn vec_i32_index_program(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
) -> raw::Program {
    let mut program = program(sources, linear, linux);
    let function = &mut program.modules[0].functions[0];
    let span = function.span;
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(5), span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span },
        raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(2), span },
    ];
    function.result = raw::TypeId(1);
    function.places = vec![
        raw::Place {
            id: raw::PlaceId(0),
            ty: raw::TypeId(5),
            span,
            kind: raw::PlaceKind::Parameter(0),
        },
        raw::Place {
            id: raw::PlaceId(1),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Parameter(2),
        },
    ];
    function.blocks[0].instructions = vec![raw::Instruction {
        result: Some(raw::ValueDefinition { id: raw::ValueId(3), ty: raw::TypeId(1), span }),
        span,
        kind: raw::InstructionKind::VecIndexCopy {
            place: raw::PlaceId(0),
            index: raw::ValueId(1),
            cleanup: raw::CleanupPlanId(0),
        },
    }];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(3), cleanup: raw::CleanupPlanId(1) };
    let actions = vec![
        raw::DropAction::DropPlace(raw::PlaceId(1)),
        raw::DropAction::DropPlace(raw::PlaceId(0)),
    ];
    function.cleanup_plans = vec![
        raw::CleanupPlan { id: raw::CleanupPlanId(0), span, actions: actions.clone() },
        raw::CleanupPlan { id: raw::CleanupPlanId(1), span, actions },
    ];
    program
}

fn vec_i32_clone_program(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
) -> raw::Program {
    let mut program = program(sources, linear, linux);
    let function = &mut program.modules[0].functions[0];
    let span = function.span;
    function.entry_export = None;
    function.parameters =
        vec![raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(5), span }];
    function.result = raw::TypeId(5);
    function.places = vec![
        raw::Place {
            id: raw::PlaceId(0),
            ty: raw::TypeId(5),
            span,
            kind: raw::PlaceKind::Parameter(0),
        },
        raw::Place {
            id: raw::PlaceId(1),
            ty: raw::TypeId(5),
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(1)),
        },
    ];
    function.blocks[0].instructions = vec![raw::Instruction {
        result: Some(raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(5), span }),
        span,
        kind: raw::InstructionKind::VecClone {
            place: raw::PlaceId(0),
            cleanup: raw::CleanupPlanId(0),
            element_cleanup: None,
        },
    }];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(1), cleanup: raw::CleanupPlanId(1) };
    let actions = vec![raw::DropAction::DropPlace(raw::PlaceId(0))];
    function.cleanup_plans = vec![
        raw::CleanupPlan { id: raw::CleanupPlanId(0), span, actions: actions.clone() },
        raw::CleanupPlan { id: raw::CleanupPlanId(1), span, actions },
    ];
    program
}

fn vec_string_clone_program(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
) -> raw::Program {
    let mut program = vec_i32_clone_program(sources, linear, linux);
    let function = &mut program.modules[0].functions[0];
    let span = function.span;
    function.parameters[0].ty = raw::TypeId(6);
    function.result = raw::TypeId(6);
    function.places[0].ty = raw::TypeId(6);
    function.places[1].ty = raw::TypeId(6);
    function.blocks[0].instructions[0].result.as_mut().expect("result").ty = raw::TypeId(6);
    let raw::InstructionKind::VecClone { element_cleanup, .. } =
        &mut function.blocks[0].instructions[0].kind
    else {
        panic!("VecClone")
    };
    *element_cleanup = Some(raw::CleanupPlanId(2));
    function.cleanup_plans.push(raw::CleanupPlan {
        id: raw::CleanupPlanId(2),
        span,
        actions: vec![
            raw::DropAction::DropVecInitializedPrefix(raw::PlaceId(1)),
            raw::DropAction::DropPlace(raw::PlaceId(0)),
        ],
    });
    program
}

fn owned_direct_call_program(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
) -> raw::Program {
    let mut program = program(sources, linear, linux);
    let span = program.modules[0].functions[0].span;
    let caller = &mut program.modules[0].functions[0];
    caller.entry_export = None;
    caller.parameters = (0..4)
        .map(|id| raw::ValueDefinition { id: raw::ValueId(id), ty: raw::TypeId(2), span })
        .chain([raw::ValueDefinition { id: raw::ValueId(4), ty: raw::TypeId(1), span }])
        .collect();
    caller.places = (0..4)
        .map(|id| raw::Place {
            id: raw::PlaceId(id),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Parameter(id),
        })
        .collect();
    caller.blocks[0].instructions = vec![raw::Instruction {
        result: Some(raw::ValueDefinition { id: raw::ValueId(5), ty: raw::TypeId(1), span }),
        span,
        kind: raw::InstructionKind::DirectCall {
            callee: raw::FunctionId { module: raw::ModuleId(0), declaration: 1 },
            arguments: vec![
                raw::CallArgument::Value(raw::ValueId(0)),
                raw::CallArgument::Value(raw::ValueId(1)),
                raw::CallArgument::Value(raw::ValueId(4)),
            ],
            cleanup: raw::CleanupPlanId(0),
        },
    }];
    caller.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(5), cleanup: raw::CleanupPlanId(1) };
    let remaining = vec![
        raw::DropAction::DropPlace(raw::PlaceId(3)),
        raw::DropAction::DropPlace(raw::PlaceId(2)),
    ];
    caller.cleanup_plans = vec![
        raw::CleanupPlan { id: raw::CleanupPlanId(0), span, actions: remaining.clone() },
        raw::CleanupPlan { id: raw::CleanupPlanId(1), span, actions: remaining },
    ];
    program.modules[0].functions.push(raw::Function {
        id: raw::FunctionId { module: raw::ModuleId(0), declaration: 1 },
        entry_export: None,
        span,
        parameters: vec![
            raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(2), span },
            raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(2), span },
            raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(1), span },
        ],
        borrow_parameters: vec![],
        result: raw::TypeId(1),
        places: vec![
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
                kind: raw::PlaceKind::Parameter(1),
            },
        ],
        blocks: vec![raw::Block {
            id: raw::BlockId(0),
            parameters: vec![],
            instructions: vec![],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Return {
                    value: raw::ValueId(2),
                    cleanup: raw::CleanupPlanId(0),
                },
            }],
        }],
        cleanup_plans: vec![raw::CleanupPlan {
            id: raw::CleanupPlanId(0),
            span,
            actions: vec![
                raw::DropAction::DropPlace(raw::PlaceId(1)),
                raw::DropAction::DropPlace(raw::PlaceId(0)),
            ],
        }],
    });
    program
}

fn scalar_call_chain(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
    functions: usize,
) -> raw::Program {
    let mut program = program(sources, linear, linux);
    let span = program.modules[0].functions[0].span;
    program.modules[0].functions = (0..functions)
        .map(|index| {
            let calls_next = index + 1 < functions;
            raw::Function {
                id: raw::FunctionId {
                    module: raw::ModuleId(0),
                    declaration: u32::try_from(index).expect("function index"),
                },
                entry_export: (index == 0).then(|| "id".to_owned()),
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
                    instructions: calls_next
                        .then(|| raw::Instruction {
                            result: Some(raw::ValueDefinition {
                                id: raw::ValueId(1),
                                ty: raw::TypeId(1),
                                span,
                            }),
                            span,
                            kind: raw::InstructionKind::DirectCall {
                                callee: raw::FunctionId {
                                    module: raw::ModuleId(0),
                                    declaration: u32::try_from(index + 1).expect("callee index"),
                                },
                                arguments: vec![raw::CallArgument::Value(raw::ValueId(0))],
                                cleanup: raw::CleanupPlanId(0),
                            },
                        })
                        .into_iter()
                        .collect(),
                    terminators: vec![raw::SpannedTerminator {
                        span,
                        kind: raw::Terminator::Return {
                            value: raw::ValueId(u32::from(calls_next)),
                            cleanup: raw::CleanupPlanId(u32::from(calls_next)),
                        },
                    }],
                }],
                cleanup_plans: (0..=usize::from(calls_next))
                    .map(|id| raw::CleanupPlan {
                        id: raw::CleanupPlanId(u32::try_from(id).expect("cleanup index")),
                        span,
                        actions: vec![],
                    })
                    .collect(),
            }
        })
        .collect();
    program
}

#[test]
fn vec_construct_failure_retains_elements_and_success_creates_one_owner() {
    let (sources, linear, linux) = authorities();
    let raw = vec_string_construct_program(&sources, &linear, &linux);
    let entry = sources.verify_file_id(0).expect("entry");
    let verified = verify(raw, &sources, entry, linear, linux).expect("VecConstruct");
    let function = verified.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let construct = block.instructions().next().expect("construct");
    assert_eq!(construct.kind(), VerifiedInstructionKind::VecConstruct);
    assert_eq!(
        construct.value_operands().map(super::ValueIdentity::index).collect::<Vec<_>>(),
        [0, 1]
    );
    assert_eq!(construct.result().expect("result owner").index(), 3);
    assert_eq!(
        construct.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
        [1, 0]
    );
    assert_eq!(
        block
            .terminator()
            .derived_drop_actions()
            .map(|action| action.root().index())
            .collect::<Vec<_>>(),
        [2]
    );
}

#[test]
fn vec_construct_duplicate_non_copy_consumption_is_rejected() {
    let (sources, linear, linux) = authorities();
    let mut raw = vec_string_construct_program(&sources, &linear, &linux);
    let raw::InstructionKind::VecConstruct { elements, .. } =
        &mut raw.modules[0].functions[0].blocks[0].instructions[0].kind
    else {
        panic!("VecConstruct fixture");
    };
    elements[1] = raw::ValueId(0);
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics =
        verify(raw, &sources, entry, linear, linux).expect_err("duplicate String consumption");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"));
}

#[test]
fn vec_clone_preserves_copy_element_source_and_authenticates_cleanup() {
    let (sources, linear, linux) = authorities();
    let raw = vec_i32_clone_program(&sources, &linear, &linux);
    let entry = sources.verify_file_id(0).expect("entry");
    let verified = verify(raw, &sources, entry, linear, linux).expect("VecClone<i32>");
    let function = verified.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let clone = block.instructions().next().expect("clone");
    assert_eq!(clone.kind(), VerifiedInstructionKind::VecClone);
    assert_eq!(clone.place_operands().next().expect("source").index(), 0);
    assert_eq!(clone.result().expect("distinct result").index(), 1);
    assert_eq!(
        clone.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
        [0]
    );
    assert_eq!(
        block
            .terminator()
            .derived_drop_actions()
            .map(|action| action.root().index())
            .collect::<Vec<_>>(),
        [0]
    );
}

#[test]
fn vec_clone_accepts_string_prefix_cleanup_and_rejects_wrong_result_or_unavailable_source() {
    let (sources, linear, linux) = authorities();
    let entry = sources.verify_file_id(0).expect("entry");

    let string_clone = vec_string_clone_program(&sources, &linear, &linux);
    let verified = verify(string_clone, &sources, entry, linear.clone(), linux.clone())
        .expect("Vec<String> clone prefix authority");
    let clone = verified
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
    assert_eq!(clone.vec_clone_element_cleanup().expect("element cleanup").index(), 2);
    let actions = clone.vec_clone_element_failure_drop_actions().collect::<Vec<_>>();
    assert_eq!(actions.len(), 2);
    assert_eq!(actions[0].kind(), VerifiedDropActionKind::VecInitializedPrefix);
    assert_eq!(actions[0].root().index(), 1);
    assert_eq!(actions[1].kind(), VerifiedDropActionKind::Place);
    assert_eq!(actions[1].root().index(), 0);

    let mut wrong_result = vec_i32_clone_program(&sources, &linear, &linux);
    wrong_result.modules[0].functions[0].blocks[0].instructions[0]
        .result
        .as_mut()
        .expect("result")
        .ty = raw::TypeId(1);
    wrong_result.modules[0].functions[0].places[1].ty = raw::TypeId(1);
    wrong_result.modules[0].functions[0].result = raw::TypeId(1);
    let diagnostics = verify(wrong_result, &sources, entry, linear.clone(), linux.clone())
        .expect_err("wrong result");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3005"));

    let mut moved = vec_i32_clone_program(&sources, &linear, &linux);
    let span = moved.modules[0].functions[0].span;
    moved.modules[0].functions[0].blocks[0].instructions.insert(
        0,
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(0) },
        },
    );
    for plan in &mut moved.modules[0].functions[0].cleanup_plans {
        plan.actions.clear();
    }
    let diagnostics =
        verify(moved, &sources, entry, linear, linux).expect_err("clone after source move");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"));
}

#[test]
fn vec_clone_rejects_cleanup_and_result_owner_forgery() {
    let (sources, linear, linux) = authorities();
    let entry = sources.verify_file_id(0).expect("entry");

    let mut foreign_cleanup = vec_i32_clone_program(&sources, &linear, &linux);
    let raw::InstructionKind::VecClone { cleanup, .. } =
        &mut foreign_cleanup.modules[0].functions[0].blocks[0].instructions[0].kind
    else {
        panic!("VecClone fixture")
    };
    *cleanup = raw::CleanupPlanId(99);
    let diagnostics = verify(foreign_cleanup, &sources, entry, linear.clone(), linux.clone())
        .expect_err("foreign cleanup");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3006"));

    let mut omitted_source = vec_i32_clone_program(&sources, &linear, &linux);
    omitted_source.modules[0].functions[0].cleanup_plans[0].actions.clear();
    assert!(
        verify(omitted_source, &sources, entry, linear.clone(), linux.clone()).is_err(),
        "prepare cleanup must retain the source owner"
    );

    let mut extra_result = vec_i32_clone_program(&sources, &linear, &linux);
    extra_result.modules[0].functions[0].cleanup_plans[0]
        .actions
        .insert(0, raw::DropAction::DropPlace(raw::PlaceId(1)));
    assert!(
        verify(extra_result, &sources, entry, linear.clone(), linux.clone()).is_err(),
        "prepare cleanup cannot include the uncommitted result owner"
    );

    let mut result_cleanup = vec_i32_clone_program(&sources, &linear, &linux);
    result_cleanup.modules[0].functions[0].cleanup_plans[1]
        .actions
        .insert(0, raw::DropAction::DropPlace(raw::PlaceId(1)));
    assert!(
        verify(result_cleanup, &sources, entry, linear.clone(), linux.clone()).is_err(),
        "return cleanup cannot drop the carried clone result"
    );

    let mut reordered = vec_i32_clone_program(&sources, &linear, &linux);
    let function = &mut reordered.modules[0].functions[0];
    let span = function.span;
    function.parameters.push(raw::ValueDefinition {
        id: raw::ValueId(1),
        ty: raw::TypeId(5),
        span,
    });
    function.places[1] = raw::Place {
        id: raw::PlaceId(1),
        ty: raw::TypeId(5),
        span,
        kind: raw::PlaceKind::Parameter(1),
    };
    function.places.push(raw::Place {
        id: raw::PlaceId(2),
        ty: raw::TypeId(5),
        span,
        kind: raw::PlaceKind::Temporary(raw::ValueId(2)),
    });
    function.blocks[0].instructions[0].result.as_mut().expect("result").id = raw::ValueId(2);
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(2), cleanup: raw::CleanupPlanId(1) };
    for plan in &mut function.cleanup_plans {
        plan.actions = vec![
            raw::DropAction::DropPlace(raw::PlaceId(0)),
            raw::DropAction::DropPlace(raw::PlaceId(1)),
        ];
    }
    assert!(
        verify(reordered, &sources, entry, linear.clone(), linux.clone()).is_err(),
        "cleanup must reverse the two live source owners"
    );

    let mut missing_result_owner = vec_i32_clone_program(&sources, &linear, &linux);
    missing_result_owner.modules[0].functions[0].places.pop();
    assert!(
        verify(missing_result_owner, &sources, entry, linear, linux).is_err(),
        "owned clone result requires its exact temporary place"
    );
}

#[test]
fn vec_string_clone_rejects_every_prefix_authority_forgery() {
    let (sources, linear, linux) = authorities();
    let entry = sources.verify_file_id(0).expect("entry");

    let mut missing = vec_string_clone_program(&sources, &linear, &linux);
    let raw::InstructionKind::VecClone { element_cleanup, .. } =
        &mut missing.modules[0].functions[0].blocks[0].instructions[0].kind
    else {
        panic!("VecClone")
    };
    *element_cleanup = None;
    assert!(verify(missing, &sources, entry, linear.clone(), linux.clone()).is_err());

    let mut foreign = vec_string_clone_program(&sources, &linear, &linux);
    let raw::InstructionKind::VecClone { element_cleanup, .. } =
        &mut foreign.modules[0].functions[0].blocks[0].instructions[0].kind
    else {
        panic!("VecClone")
    };
    *element_cleanup = Some(raw::CleanupPlanId(99));
    assert!(verify(foreign, &sources, entry, linear.clone(), linux.clone()).is_err());

    for forged in [
        vec![raw::DropAction::DropPlace(raw::PlaceId(0))],
        vec![
            raw::DropAction::DropVecInitializedPrefix(raw::PlaceId(0)),
            raw::DropAction::DropPlace(raw::PlaceId(0)),
        ],
        vec![
            raw::DropAction::DropPlace(raw::PlaceId(0)),
            raw::DropAction::DropVecInitializedPrefix(raw::PlaceId(1)),
        ],
        vec![
            raw::DropAction::DropVecInitializedPrefix(raw::PlaceId(1)),
            raw::DropAction::DropPlace(raw::PlaceId(0)),
            raw::DropAction::DropPlace(raw::PlaceId(1)),
        ],
    ] {
        let mut program = vec_string_clone_program(&sources, &linear, &linux);
        program.modules[0].functions[0].cleanup_plans[2].actions = forged;
        assert!(
            verify(program, &sources, entry, linear.clone(), linux.clone()).is_err(),
            "missing, wrong-root, reordered, and extra prefix claims fail closed"
        );
    }

    let mut ordinary_prefix = vec_i32_clone_program(&sources, &linear, &linux);
    ordinary_prefix.modules[0].functions[0].cleanup_plans[0].actions[0] =
        raw::DropAction::DropVecInitializedPrefix(raw::PlaceId(0));
    assert!(
        verify(ordinary_prefix, &sources, entry, linear.clone(), linux.clone()).is_err(),
        "ordinary allocation cleanup cannot forge a dynamic prefix action"
    );

    let mut copy_with_element_cleanup = vec_i32_clone_program(&sources, &linear, &linux);
    let function = &mut copy_with_element_cleanup.modules[0].functions[0];
    let span = function.span;
    let raw::InstructionKind::VecClone { element_cleanup, .. } =
        &mut function.blocks[0].instructions[0].kind
    else {
        panic!("VecClone")
    };
    *element_cleanup = Some(raw::CleanupPlanId(2));
    function.cleanup_plans.push(raw::CleanupPlan {
        id: raw::CleanupPlanId(2),
        span,
        actions: vec![raw::DropAction::DropVecInitializedPrefix(raw::PlaceId(1))],
    });
    assert!(verify(copy_with_element_cleanup, &sources, entry, linear, linux).is_err());
}

#[test]
fn vec_index_copy_accepts_copy_and_exposes_complete_bounds_cleanup() {
    let (sources, linear, linux) = authorities();
    let raw = vec_i32_index_program(&sources, &linear, &linux);
    let entry = sources.verify_file_id(0).expect("entry");
    let verified = verify(raw, &sources, entry, linear, linux).expect("VecIndexCopy<i32>");
    let function = verified.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let index = block.instructions().next().expect("index");
    assert_eq!(index.kind(), VerifiedInstructionKind::VecIndexCopy);
    assert_eq!(index.place_operands().next().expect("vector").index(), 0);
    assert_eq!(index.value_operands().next().expect("index value").index(), 1);
    assert_eq!(index.result().expect("copy result").index(), 3);
    assert_eq!(
        index.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
        [1, 0]
    );
    assert_eq!(
        block
            .terminator()
            .derived_drop_actions()
            .map(|action| action.root().index())
            .collect::<Vec<_>>(),
        [1, 0]
    );
}

#[test]
fn vec_index_copy_rejects_non_copy_element_wrong_index_and_wrong_result() {
    let (sources, linear, linux) = authorities();
    let entry = sources.verify_file_id(0).expect("entry");

    let mut string_element = vec_i32_index_program(&sources, &linear, &linux);
    let function = &mut string_element.modules[0].functions[0];
    let span = function.span;
    function.parameters[0].ty = raw::TypeId(6);
    function.places[0].ty = raw::TypeId(6);
    function.blocks[0].instructions[0].result.as_mut().expect("result").ty = raw::TypeId(2);
    function.result = raw::TypeId(2);
    function.places.push(raw::Place {
        id: raw::PlaceId(2),
        ty: raw::TypeId(2),
        span,
        kind: raw::PlaceKind::Temporary(raw::ValueId(3)),
    });
    let diagnostics = verify(string_element, &sources, entry, linear.clone(), linux.clone())
        .expect_err("String elements are not Copy");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3005"));

    let mut wrong_index = vec_i32_index_program(&sources, &linear, &linux);
    wrong_index.modules[0].functions[0].parameters[1].ty = raw::TypeId(0);
    let diagnostics = verify(wrong_index, &sources, entry, linear.clone(), linux.clone())
        .expect_err("bool index");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3005"));

    let mut wrong_result = vec_i32_index_program(&sources, &linear, &linux);
    let function = &mut wrong_result.modules[0].functions[0];
    function.blocks[0].instructions[0].result.as_mut().expect("result").ty = raw::TypeId(0);
    function.result = raw::TypeId(0);
    let diagnostics = verify(wrong_result, &sources, entry, linear, linux)
        .expect_err("result differs from element type");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3005"));
}

#[test]
fn vec_cleanup_identity_order_and_type_mutations_are_rejected() {
    let (sources, linear, linux) = authorities();
    let entry = sources.verify_file_id(0).expect("entry");

    let mut wrong_order = vec_string_construct_program(&sources, &linear, &linux);
    wrong_order.modules[0].functions[0].cleanup_plans[0].actions.swap(0, 1);
    let diagnostics = verify(wrong_order, &sources, entry, linear.clone(), linux.clone())
        .expect_err("wrong prepare cleanup order");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3012"));

    let mut reused = vec_i32_index_program(&sources, &linear, &linux);
    let raw::Terminator::Return { cleanup, .. } =
        &mut reused.modules[0].functions[0].blocks[0].terminators[0].kind
    else {
        panic!("return fixture");
    };
    *cleanup = raw::CleanupPlanId(0);
    let diagnostics = verify(reused, &sources, entry, linear.clone(), linux.clone())
        .expect_err("cleanup plan reused by two sites");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3012"));

    let mut foreign_place = vec_i32_index_program(&sources, &linear, &linux);
    let raw::InstructionKind::VecIndexCopy { place, .. } =
        &mut foreign_place.modules[0].functions[0].blocks[0].instructions[0].kind
    else {
        panic!("VecIndexCopy fixture");
    };
    *place = raw::PlaceId(99);
    let diagnostics = verify(foreign_place, &sources, entry, linear.clone(), linux.clone())
        .expect_err("foreign place identity");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3006"));

    let mut wrong_type = vec_string_construct_program(&sources, &linear, &linux);
    let function = &mut wrong_type.modules[0].functions[0];
    function.blocks[0].instructions[0].result.as_mut().expect("result").ty = raw::TypeId(5);
    function.places[2].ty = raw::TypeId(5);
    let diagnostics = verify(wrong_type, &sources, entry, linear, linux)
        .expect_err("Vec<i32> result with String elements");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3005"));
}

#[allow(clippy::too_many_lines)]
fn alternate_branch_owner_program(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
) -> raw::Program {
    let mut program = program(sources, linear, linux);
    let function = &mut program.modules[0].functions[0];
    let span = function.span;
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(2), span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(2), span },
        raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(0), span },
    ];
    function.result = raw::TypeId(0);
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
            kind: raw::PlaceKind::Parameter(1),
        },
        raw::Place {
            id: raw::PlaceId(2),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(3)),
        },
        raw::Place {
            id: raw::PlaceId(3),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(4)),
        },
        raw::Place {
            id: raw::PlaceId(4),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(5)),
        },
    ];
    let edge = |target, arguments| raw::Edge { target: raw::BlockId(target), arguments };
    function.blocks = vec![
        raw::Block {
            id: raw::BlockId(0),
            parameters: vec![],
            instructions: vec![],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Branch {
                    condition: raw::ValueId(2),
                    when_true: edge(1, vec![raw::ValueId(0)]),
                    when_false: edge(2, vec![raw::ValueId(1)]),
                },
            }],
        },
        raw::Block {
            id: raw::BlockId(1),
            parameters: vec![raw::ValueDefinition {
                id: raw::ValueId(3),
                ty: raw::TypeId(2),
                span,
            }],
            instructions: vec![raw::Instruction {
                result: None,
                span,
                kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(1) },
            }],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Jump(edge(3, vec![raw::ValueId(3)])),
            }],
        },
        raw::Block {
            id: raw::BlockId(2),
            parameters: vec![raw::ValueDefinition {
                id: raw::ValueId(4),
                ty: raw::TypeId(2),
                span,
            }],
            instructions: vec![raw::Instruction {
                result: None,
                span,
                kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(0) },
            }],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Jump(edge(3, vec![raw::ValueId(4)])),
            }],
        },
        raw::Block {
            id: raw::BlockId(3),
            parameters: vec![raw::ValueDefinition {
                id: raw::ValueId(5),
                ty: raw::TypeId(2),
                span,
            }],
            instructions: vec![],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Return {
                    value: raw::ValueId(2),
                    cleanup: raw::CleanupPlanId(0),
                },
            }],
        },
    ];
    function.cleanup_plans[0].actions = vec![raw::DropAction::DropPlace(raw::PlaceId(4))];
    program
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
fn owned_utf8_literal_verifies_and_exposes_only_immutable_bytes() {
    let (sources, linear, linux) = authorities();
    let bytes = "snowman: ☃".as_bytes().to_vec();
    let raw = string_literal_program(&sources, &linear, &linux, bytes.clone());
    let entry = sources.verify_file_id(0).expect("entry");
    let verified = verify(raw, &sources, entry, linear, linux).expect("valid UTF-8 literal");
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
        .expect("literal");
    assert_eq!(instruction.kind(), VerifiedInstructionKind::StringFromUtf8);
    assert_eq!(instruction.string_utf8_bytes(), Some(bytes.as_slice()));
    assert_eq!(instruction.cleanup().expect("cleanup").index(), 0);
    let function = verified.modules().next().expect("module").functions().next().expect("function");
    assert!(!function.places().next().expect("String result place").is_copy());
    let sites = function.cleanup_plans().map(super::VerifiedCleanupPlan::site).collect::<Vec<_>>();
    assert_eq!(sites[0].block().index(), 0);
    assert_eq!(sites[0].instruction_index(), Some(0));
    assert_eq!(sites[0].role(), VerifiedCleanupRole::PrepareFailure);
    assert_eq!(sites[1].instruction_index(), None);
    assert_eq!(sites[1].role(), VerifiedCleanupRole::Return);
}

#[test]
fn owned_utf8_literal_rejects_invalid_utf8_result_and_cleanup() {
    let (sources, linear, linux) = authorities();
    let entry = sources.verify_file_id(0).expect("entry");

    let invalid_utf8 = string_literal_program(&sources, &linear, &linux, vec![0xff]);
    let diagnostics = verify(invalid_utf8, &sources, entry, linear.clone(), linux.clone())
        .expect_err("invalid UTF-8");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3005"));

    let mut wrong_result = string_literal_program(&sources, &linear, &linux, b"x".to_vec());
    wrong_result.modules[0].functions[0].result = raw::TypeId(1);
    wrong_result.modules[0].functions[0].blocks[0].instructions[0]
        .result
        .as_mut()
        .expect("result")
        .ty = raw::TypeId(1);
    wrong_result.modules[0].functions[0].places[0].ty = raw::TypeId(1);
    let diagnostics = verify(wrong_result, &sources, entry, linear.clone(), linux.clone())
        .expect_err("non-String result");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3005"));

    let mut wrong_cleanup = string_literal_program(&sources, &linear, &linux, b"x".to_vec());
    let raw::InstructionKind::StringFromUtf8 { cleanup, .. } =
        &mut wrong_cleanup.modules[0].functions[0].blocks[0].instructions[0].kind
    else {
        panic!("String literal instruction");
    };
    *cleanup = raw::CleanupPlanId(2);
    let diagnostics = verify(wrong_cleanup, &sources, entry, linear.clone(), linux.clone())
        .expect_err("foreign cleanup plan");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3006"));

    let mut reused_cleanup = string_literal_program(&sources, &linear, &linux, b"x".to_vec());
    if let raw::Terminator::Return { cleanup, .. } =
        &mut reused_cleanup.modules[0].functions[0].blocks[0].terminators[0].kind
    {
        *cleanup = raw::CleanupPlanId(0);
    }
    let diagnostics = verify(reused_cleanup, &sources, entry, linear, linux)
        .expect_err("one plan cannot authorize two sites");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3012"));
}

#[test]
fn generic_clone_is_limited_to_structural_aggregate_categories() {
    for (ty, label) in [
        (raw::TypeId(2), "String"),
        (raw::TypeId(6), "Vec"),
        (raw::TypeId(7), "Shared"),
        (raw::TypeId(8), "Weak"),
    ] {
        let (sources, linear, linux) = authorities();
        let raw = string_clone_program(&sources, &linear, &linux, ty, true);
        let entry = sources.verify_file_id(0).expect("entry");
        let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err(label);
        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3005"),
            "generic clone unexpectedly admitted {label}: {diagnostics:?}"
        );
    }

    let (sources, linear, linux) = authorities();
    let raw = string_bearing_aggregate_clone_program(&sources, &linear, &linux, raw::TypeId(3));
    let entry = sources.verify_file_id(0).expect("entry");
    verify(raw, &sources, entry, linear, linux).expect("struct containing String");

    let (sources, linear, linux) = authorities();
    let raw = string_bearing_aggregate_clone_program(&sources, &linear, &linux, raw::TypeId(4));
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux)
        .expect_err("unrefined enum parameter must not authorize structural clone");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3013"));

    for (runtime_child, label) in [(0, "Vec child"), (1, "Shared child"), (2, "Weak child")] {
        let (sources, linear, linux, ty) = runtime_child_clone_authorities(runtime_child);
        let mut raw = string_clone_program(&sources, &linear, &linux, ty, true);
        raw.modules[0].data_declarations = 1;
        let entry = sources.verify_file_id(0).expect("entry");
        let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err(label);
        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3005"),
            "generic clone unexpectedly admitted {label}: {diagnostics:?}"
        );
    }

    let (sources, linear, linux, _) = copy_clone_authorities();
    let ty = linear
        .types()
        .find(|ty| ty.nominal_identity() == Some((0, 0)))
        .map(|ty| raw::TypeId(ty.id().index()))
        .expect("Copy struct");
    let mut raw = string_clone_program(&sources, &linear, &linux, ty, true);
    raw.modules[0].data_declarations = 3;
    for plan in &mut raw.modules[0].functions[0].cleanup_plans {
        plan.actions.clear();
    }
    let entry = sources.verify_file_id(0).expect("entry");
    let verified =
        verify(raw, &sources, entry, linear, linux).expect("recursively Copy structural clone");
    let clone = verified
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
    assert_eq!(clone.aggregate_clone_fallible_leaf_count(), None);
}

#[test]
#[allow(clippy::too_many_lines)]
fn string_bearing_aggregate_clone_cleanup_is_exact_and_fail_closed() {
    let (sources, linear, linux) = authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let baseline =
        string_bearing_aggregate_clone_program(&sources, &linear, &linux, raw::TypeId(3));
    let verified = verify(baseline.clone(), &sources, entry, linear.clone(), linux.clone())
        .expect("String-bearing Struct clone");
    let function = verified.modules().next().expect("module").functions().next().expect("function");
    let clone = function.blocks().next().expect("block").instructions().next().expect("clone");
    assert_eq!(
        clone
            .aggregate_clone_element_cleanup()
            .and_then(|plan| { function.cleanup_plans().find(|candidate| candidate.id() == plan) })
            .map(super::VerifiedCleanupPlan::site)
            .map(super::VerifiedCleanupSite::role),
        Some(VerifiedCleanupRole::AggregateCloneElementFailure),
    );
    assert_eq!(
        clone
            .aggregate_clone_element_failure_drop_actions()
            .map(|action| (action.kind(), action.root().index()))
            .collect::<Vec<_>>(),
        vec![
            (VerifiedDropActionKind::AggregateInitializedPrefix, 1),
            (VerifiedDropActionKind::Place, 0),
        ]
    );

    let assert_rejected = |raw: raw::Program, label: &str| {
        let diagnostics =
            verify(raw, &sources, entry, linear.clone(), linux.clone()).expect_err(label);
        assert!(
            diagnostics.iter().any(|diagnostic| {
                matches!(
                    diagnostic.code(),
                    "ZRYNA-I3005" | "ZRYNA-I3006" | "ZRYNA-I3012" | "ZRYNA-I3013"
                )
            }),
            "{label}: {diagnostics:?}",
        );
    };

    let mut missing = baseline.clone();
    let raw::InstructionKind::ClonePlace { element_cleanup, .. } =
        &mut missing.modules[0].functions[0].blocks[0].instructions[0].kind
    else {
        panic!("ClonePlace")
    };
    *element_cleanup = None;
    assert_rejected(missing, "missing element cleanup");

    let mut foreign = baseline.clone();
    let raw::InstructionKind::ClonePlace { element_cleanup, .. } =
        &mut foreign.modules[0].functions[0].blocks[0].instructions[0].kind
    else {
        panic!("ClonePlace")
    };
    *element_cleanup = Some(raw::CleanupPlanId(99));
    assert_rejected(foreign, "foreign element cleanup");

    let mut reused = baseline.clone();
    let raw::InstructionKind::ClonePlace { element_cleanup, .. } =
        &mut reused.modules[0].functions[0].blocks[0].instructions[0].kind
    else {
        panic!("ClonePlace")
    };
    *element_cleanup = Some(raw::CleanupPlanId(0));
    assert_rejected(reused, "reused element cleanup");

    for (actions, label) in [
        (
            vec![raw::DropAction::DropAggregateInitializedPrefix(raw::PlaceId(0))],
            "wrong result owner and missing roots",
        ),
        (
            vec![
                raw::DropAction::DropPlace(raw::PlaceId(0)),
                raw::DropAction::DropAggregateInitializedPrefix(raw::PlaceId(1)),
            ],
            "reordered roots",
        ),
        (
            vec![
                raw::DropAction::DropAggregateInitializedPrefix(raw::PlaceId(1)),
                raw::DropAction::DropPlace(raw::PlaceId(0)),
                raw::DropAction::DropPlace(raw::PlaceId(1)),
            ],
            "extra root",
        ),
    ] {
        let mut raw = baseline.clone();
        raw.modules[0].functions[0].cleanup_plans[2].actions = actions;
        assert_rejected(raw, label);
    }

    let mut prefix_in_ordinary = baseline.clone();
    prefix_in_ordinary.modules[0].functions[0].cleanup_plans[0].actions =
        vec![raw::DropAction::DropAggregateInitializedPrefix(raw::PlaceId(0))];
    assert_rejected(prefix_in_ordinary, "prefix in ordinary cleanup");

    let (copy_sources, copy_linear, copy_linux, _) = copy_clone_authorities();
    let copy_ty = copy_linear
        .types()
        .find(|ty| ty.nominal_identity() == Some((0, 0)))
        .map(|ty| raw::TypeId(ty.id().index()))
        .expect("Copy struct");
    let mut copy = string_clone_program(&copy_sources, &copy_linear, &copy_linux, copy_ty, true);
    copy.modules[0].data_declarations = 3;
    for plan in &mut copy.modules[0].functions[0].cleanup_plans {
        plan.actions.clear();
    }
    let copy_span = copy.modules[0].functions[0].span;
    copy.modules[0].functions[0].cleanup_plans.push(raw::CleanupPlan {
        id: raw::CleanupPlanId(2),
        span: copy_span,
        actions: vec![],
    });
    let raw::InstructionKind::ClonePlace { element_cleanup, .. } =
        &mut copy.modules[0].functions[0].blocks[0].instructions[0].kind
    else {
        panic!("ClonePlace")
    };
    *element_cleanup = Some(raw::CleanupPlanId(2));
    let copy_entry = copy_sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(copy, &copy_sources, copy_entry, copy_linear, copy_linux)
        .expect_err("Copy aggregate with element cleanup");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3005"));

    let (payloadless_sources, payloadless_linear, payloadless_linux) =
        payloadless_enum_authorities();
    let payloadless = payloadless_active_enum_clone_program(
        &payloadless_sources,
        &payloadless_linear,
        &payloadless_linux,
    );
    let payloadless_entry = payloadless_sources.verify_file_id(0).expect("entry");
    let verified = verify(
        payloadless,
        &payloadless_sources,
        payloadless_entry,
        payloadless_linear,
        payloadless_linux,
    )
    .expect("payloadless active enum clone");
    let clone = verified
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
        .find(|instruction| instruction.kind() == VerifiedInstructionKind::ClonePlace)
        .expect("clone");
    assert_eq!(clone.aggregate_clone_fallible_leaf_count(), Some(0));
    let prefix = clone
        .aggregate_clone_element_failure_drop_actions()
        .next()
        .expect("aggregate prefix action");
    assert_eq!(prefix.kind(), VerifiedDropActionKind::AggregateInitializedPrefix);
    assert_eq!(prefix.active_variant(), Some(0));
    assert_eq!(
        prefix
            .active_variants()
            .find(|variant| variant.place() == prefix.root())
            .map(VerifiedActiveVariant::variant),
        Some(0),
    );
}

#[test]
fn infallible_aggregate_constructs_reject_prepare_failure_cleanup_claims() {
    for (ty, is_enum, label) in
        [(raw::TypeId(3), false, "StructConstruct"), (raw::TypeId(4), true, "EnumConstruct")]
    {
        let (sources, linear, linux) = authorities();
        let baseline = owned_construct_program(&sources, &linear, &linux, ty, is_enum);
        let entry = sources.verify_file_id(0).expect("entry");
        verify(baseline.clone(), &sources, entry, linear.clone(), linux.clone())
            .expect("infallible aggregate construction");
        let mut mutated = baseline;
        let function = &mut mutated.modules[0].functions[0];
        let span = function.span;
        function.cleanup_plans.push(raw::CleanupPlan {
            id: raw::CleanupPlanId(1),
            span,
            actions: vec![raw::DropAction::DropPlace(raw::PlaceId(0))],
        });
        match &mut function.blocks[0].instructions[0].kind {
            raw::InstructionKind::StructConstruct { cleanup, .. }
            | raw::InstructionKind::EnumConstruct { cleanup, .. } => {
                *cleanup = Some(raw::CleanupPlanId(1));
            }
            _ => panic!("aggregate construction"),
        }
        let diagnostics = verify(mutated, &sources, entry, linear, linux).expect_err(label);
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3005"));
    }

    let (sources, linear, linux, _) = copy_clone_authorities();
    let array = linear
        .types()
        .find(|ty| ty.category() == zryna_layout::TypeCategory::FixedArray)
        .expect("fixed array");
    let element = array.referenced_type().expect("array element");
    let array_ty = raw::TypeId(array.id().index());
    let element_ty = raw::TypeId(element.index());
    let mut baseline = program(&sources, &linear, &linux);
    baseline.modules[0].data_declarations = 3;
    let function = &mut baseline.modules[0].functions[0];
    let span = function.span;
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: element_ty, span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: element_ty, span },
    ];
    function.result = array_ty;
    function.places.clear();
    function.blocks[0].instructions = vec![raw::Instruction {
        result: Some(raw::ValueDefinition { id: raw::ValueId(2), ty: array_ty, span }),
        span,
        kind: raw::InstructionKind::FixedArrayConstruct {
            elements: vec![raw::ValueId(0), raw::ValueId(1)],
            cleanup: None,
        },
    }];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(2), cleanup: raw::CleanupPlanId(0) };
    function.cleanup_plans[0].actions.clear();
    let entry = sources.verify_file_id(0).expect("entry");
    verify(baseline.clone(), &sources, entry, linear.clone(), linux.clone())
        .expect("infallible fixed-array construction");
    baseline.modules[0].functions[0].cleanup_plans.push(raw::CleanupPlan {
        id: raw::CleanupPlanId(1),
        span,
        actions: Vec::new(),
    });
    let raw::InstructionKind::FixedArrayConstruct { cleanup, .. } =
        &mut baseline.modules[0].functions[0].blocks[0].instructions[0].kind
    else {
        panic!("fixed-array construction")
    };
    *cleanup = Some(raw::CleanupPlanId(1));
    let diagnostics =
        verify(baseline, &sources, entry, linear, linux).expect_err("FixedArrayConstruct");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3005"));
}

#[test]
fn string_clone_failure_retains_source_and_success_creates_distinct_owner() {
    let (sources, linear, linux) = authorities();
    let raw = string_clone_program(&sources, &linear, &linux, raw::TypeId(2), false);
    let entry = sources.verify_file_id(0).expect("entry");
    let verified = verify(raw, &sources, entry, linear, linux).expect("StringClone");
    let function = verified.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let clone = block.instructions().next().expect("clone");
    assert_eq!(clone.kind(), VerifiedInstructionKind::StringClone);
    assert_eq!(clone.place_operands().next().expect("source").index(), 0);
    assert_eq!(clone.result().expect("result").index(), 2);
    assert_eq!(
        clone.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
        [0]
    );
    assert_eq!(
        block
            .terminator()
            .derived_drop_actions()
            .map(|action| action.root().index())
            .collect::<Vec<_>>(),
        [1, 0]
    );
}

#[test]
fn string_concat_retains_sources_creates_result_and_accepts_same_place() {
    for same_place in [false, true] {
        let (sources, linear, linux) = authorities();
        let raw = string_concat_program(&sources, &linear, &linux, same_place);
        let entry = sources.verify_file_id(0).expect("entry");
        let verified = verify(raw, &sources, entry, linear, linux).expect("StringConcat");
        let function =
            verified.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let concat = block.instructions().next().expect("concat");
        assert_eq!(concat.kind(), VerifiedInstructionKind::StringConcat);
        let operands = concat.place_operands().map(super::PlaceIdentity::index).collect::<Vec<_>>();
        assert_eq!(operands, if same_place { vec![0, 0] } else { vec![0, 1] });
        assert_eq!(concat.result().expect("result").index(), 3);
        assert_eq!(
            concat.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
            [1, 0]
        );
        assert_eq!(
            block
                .terminator()
                .derived_drop_actions()
                .map(|action| action.root().index())
                .collect::<Vec<_>>(),
            [2, 1, 0]
        );
    }
}

#[test]
fn string_clone_rejects_moved_uninitialized_and_exclusively_borrowed_sources() {
    let (sources, linear, linux) = authorities();
    let entry = sources.verify_file_id(0).expect("entry");

    let mut moved = string_clone_program(&sources, &linear, &linux, raw::TypeId(2), false);
    let function = &mut moved.modules[0].functions[0];
    let span = function.span;
    function.places.push(raw::Place {
        id: raw::PlaceId(2),
        ty: raw::TypeId(2),
        span,
        kind: raw::PlaceKind::Temporary(raw::ValueId(2)),
    });
    function.places[1].kind = raw::PlaceKind::Temporary(raw::ValueId(3));
    function.blocks[0].instructions[0].result.as_mut().expect("clone result").id = raw::ValueId(3);
    function.blocks[0].instructions.insert(
        0,
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(2), span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(0) },
        },
    );
    function.cleanup_plans[0].actions = vec![raw::DropAction::DropPlace(raw::PlaceId(2))];
    function.cleanup_plans[1].actions = vec![
        raw::DropAction::DropPlace(raw::PlaceId(1)),
        raw::DropAction::DropPlace(raw::PlaceId(2)),
    ];
    let diagnostics =
        verify(moved, &sources, entry, linear.clone(), linux.clone()).expect_err("moved source");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"));

    let mut uninitialized = string_clone_program(&sources, &linear, &linux, raw::TypeId(2), false);
    let function = &mut uninitialized.modules[0].functions[0];
    function.parameters.remove(0);
    function.parameters[0].id = raw::ValueId(0);
    function.places[0].kind = raw::PlaceKind::Local(0);
    function.places[1].kind = raw::PlaceKind::Temporary(raw::ValueId(1));
    function.blocks[0].instructions[0].result.as_mut().expect("clone result").id = raw::ValueId(1);
    if let raw::Terminator::Return { value, .. } = &mut function.blocks[0].terminators[0].kind {
        *value = raw::ValueId(0);
    }
    function.cleanup_plans[0].actions.clear();
    function.cleanup_plans[1].actions = vec![raw::DropAction::DropPlace(raw::PlaceId(1))];
    let diagnostics = verify(uninitialized, &sources, entry, linear.clone(), linux.clone())
        .expect_err("uninitialized source");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"));

    let mut borrowed = string_clone_program(&sources, &linear, &linux, raw::TypeId(2), false);
    let function = &mut borrowed.modules[0].functions[0];
    function.blocks[0].instructions.insert(
        0,
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
    );
    function.blocks[0].instructions.push(raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::EndBorrow { borrow: raw::BorrowId(0) },
    });
    let diagnostics =
        verify(borrowed, &sources, entry, linear, linux).expect_err("borrowed source");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"));
}

#[test]
fn string_concat_rejects_moved_uninitialized_and_exclusively_borrowed_sources() {
    let (sources, linear, linux) = authorities();
    let entry = sources.verify_file_id(0).expect("entry");

    let mut moved = string_concat_program(&sources, &linear, &linux, false);
    let function = &mut moved.modules[0].functions[0];
    let span = function.span;
    function.places[2].kind = raw::PlaceKind::Temporary(raw::ValueId(4));
    function.blocks[0].instructions[0].result.as_mut().expect("concat result").id = raw::ValueId(4);
    function.places.push(raw::Place {
        id: raw::PlaceId(3),
        ty: raw::TypeId(2),
        span,
        kind: raw::PlaceKind::Temporary(raw::ValueId(3)),
    });
    function.blocks[0].instructions.insert(
        0,
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(3), ty: raw::TypeId(2), span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(0) },
        },
    );
    function.cleanup_plans[0].actions = vec![
        raw::DropAction::DropPlace(raw::PlaceId(1)),
        raw::DropAction::DropPlace(raw::PlaceId(3)),
    ];
    function.cleanup_plans[1].actions = vec![
        raw::DropAction::DropPlace(raw::PlaceId(2)),
        raw::DropAction::DropPlace(raw::PlaceId(1)),
        raw::DropAction::DropPlace(raw::PlaceId(3)),
    ];
    let diagnostics =
        verify(moved, &sources, entry, linear.clone(), linux.clone()).expect_err("moved source");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"));

    let mut uninitialized = string_concat_program(&sources, &linear, &linux, false);
    let function = &mut uninitialized.modules[0].functions[0];
    function.places.push(raw::Place {
        id: raw::PlaceId(3),
        ty: raw::TypeId(2),
        span,
        kind: raw::PlaceKind::Local(0),
    });
    let raw::InstructionKind::StringConcat { left, .. } =
        &mut function.blocks[0].instructions[0].kind
    else {
        panic!("concat")
    };
    *left = raw::PlaceId(3);
    let diagnostics = verify(uninitialized, &sources, entry, linear.clone(), linux.clone())
        .expect_err("uninitialized source");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"));

    let mut borrowed = string_concat_program(&sources, &linear, &linux, false);
    let function = &mut borrowed.modules[0].functions[0];
    function.blocks[0].instructions.insert(
        0,
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
    );
    function.blocks[0].instructions.push(raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::EndBorrow { borrow: raw::BorrowId(0) },
    });
    let diagnostics =
        verify(borrowed, &sources, entry, linear, linux).expect_err("borrowed source");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"));
}

#[test]
fn string_clone_cleanup_must_match_precommit_owner_state() {
    let (sources, linear, linux) = authorities();
    let mut raw = string_clone_program(&sources, &linear, &linux, raw::TypeId(2), false);
    raw.modules[0].functions[0].cleanup_plans[0].actions.clear();
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics =
        verify(raw, &sources, entry, linear, linux).expect_err("incomplete prepare cleanup");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3012"));

    let (sources, linear, linux) = authorities();
    let mut raw = string_concat_program(&sources, &linear, &linux, false);
    raw.modules[0].functions[0].cleanup_plans[0].actions.clear();
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics =
        verify(raw, &sources, entry, linear, linux).expect_err("incomplete concat cleanup");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3012"));
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
    let raw::Terminator::Return { cleanup, .. } = &mut block.terminators[0].kind else {
        panic!("return terminator");
    };
    *cleanup = raw::CleanupPlanId(1);
    raw.modules[0].functions[0].blocks.push(block);
    let span = raw.modules[0].functions[0].span;
    raw.modules[0].functions[0].cleanup_plans.push(raw::CleanupPlan {
        id: raw::CleanupPlanId(1),
        span,
        actions: vec![],
    });
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
fn returned_owner_is_excluded_before_exit_cleanup() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.entry_export = None;
    function.parameters =
        vec![raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(2), span }];
    function.result = raw::TypeId(2);
    function.places = vec![raw::Place {
        id: raw::PlaceId(0),
        ty: raw::TypeId(2),
        span,
        kind: raw::PlaceKind::Parameter(0),
    }];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(0), cleanup: raw::CleanupPlanId(0) };
    let entry = sources.verify_file_id(0).expect("entry");
    let verified = verify(raw, &sources, entry, linear, linux).expect("returned owner transfers");
    assert_eq!(
        verified
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
            .count(),
        0
    );
}

#[test]
fn return_preserves_exact_reverse_order_of_remaining_owners() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.entry_export = None;
    function.parameters = (0..3)
        .map(|index| raw::ValueDefinition { id: raw::ValueId(index), ty: raw::TypeId(2), span })
        .collect();
    function.result = raw::TypeId(2);
    function.places = (0..3)
        .map(|index| raw::Place {
            id: raw::PlaceId(index),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Parameter(index),
        })
        .collect();
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(1), cleanup: raw::CleanupPlanId(0) };
    function.cleanup_plans[0].actions = vec![
        raw::DropAction::DropPlace(raw::PlaceId(2)),
        raw::DropAction::DropPlace(raw::PlaceId(0)),
    ];
    let entry = sources.verify_file_id(0).expect("entry");
    let verified = verify(raw, &sources, entry, linear, linux).expect("exact remaining stack");
    let roots = verified
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
        .map(|action| action.root().index())
        .collect::<Vec<_>>();
    assert_eq!(roots, [2, 0]);
}

#[test]
fn alternate_branch_owners_rename_into_one_block_owner() {
    let (sources, linear, linux) = authorities();
    let raw = alternate_branch_owner_program(&sources, &linear, &linux);
    let entry = sources.verify_file_id(0).expect("entry");
    verify(raw, &sources, entry, linear, linux).expect("exact owner rename at join");
}

#[test]
fn unequal_branch_owner_stacks_are_rejected() {
    let (sources, linear, linux) = authorities();
    let mut raw = alternate_branch_owner_program(&sources, &linear, &linux);
    raw.modules[0].functions[0].blocks[1].instructions.clear();
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("unequal join stacks");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"));
}

#[test]
fn leaked_owner_on_loop_backedge_is_rejected() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(2), span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(0), span },
    ];
    function.result = raw::TypeId(0);
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
            kind: raw::PlaceKind::Temporary(raw::ValueId(2)),
        },
        raw::Place {
            id: raw::PlaceId(2),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(3)),
        },
    ];
    function.blocks = vec![
        raw::Block {
            id: raw::BlockId(0),
            parameters: vec![],
            instructions: vec![],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Jump(raw::Edge {
                    target: raw::BlockId(1),
                    arguments: vec![raw::ValueId(0)],
                }),
            }],
        },
        raw::Block {
            id: raw::BlockId(1),
            parameters: vec![raw::ValueDefinition {
                id: raw::ValueId(2),
                ty: raw::TypeId(2),
                span,
            }],
            instructions: vec![raw::Instruction {
                result: None,
                span,
                kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(1) },
            }],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Branch {
                    condition: raw::ValueId(1),
                    when_true: raw::Edge {
                        target: raw::BlockId(1),
                        arguments: vec![raw::ValueId(2)],
                    },
                    when_false: raw::Edge {
                        target: raw::BlockId(2),
                        arguments: vec![raw::ValueId(2)],
                    },
                },
            }],
        },
        raw::Block {
            id: raw::BlockId(2),
            parameters: vec![raw::ValueDefinition {
                id: raw::ValueId(3),
                ty: raw::TypeId(2),
                span,
            }],
            instructions: vec![],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Return {
                    value: raw::ValueId(1),
                    cleanup: raw::CleanupPlanId(0),
                },
            }],
        },
    ];
    function.cleanup_plans[0].actions = vec![raw::DropAction::DropPlace(raw::PlaceId(2))];
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("leaked backedge");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"));
}

#[test]
fn vec_push_failure_retains_both_owners_and_success_consumes_argument_once() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(6), span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(2), span },
        raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(1), span },
    ];
    function.result = raw::TypeId(1);
    function.places = vec![
        raw::Place {
            id: raw::PlaceId(0),
            ty: raw::TypeId(6),
            span,
            kind: raw::PlaceKind::Parameter(0),
        },
        raw::Place {
            id: raw::PlaceId(1),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Parameter(1),
        },
    ];
    function.blocks[0].instructions = vec![raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::VecPush {
            vector: raw::PlaceId(0),
            value: raw::ValueId(1),
            cleanup: raw::CleanupPlanId(0),
        },
    }];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(2), cleanup: raw::CleanupPlanId(1) };
    function.cleanup_plans = vec![
        raw::CleanupPlan {
            id: raw::CleanupPlanId(0),
            span,
            actions: vec![
                raw::DropAction::DropPlace(raw::PlaceId(1)),
                raw::DropAction::DropPlace(raw::PlaceId(0)),
            ],
        },
        raw::CleanupPlan {
            id: raw::CleanupPlanId(1),
            span,
            actions: vec![raw::DropAction::DropPlace(raw::PlaceId(0))],
        },
    ];
    let entry = sources.verify_file_id(0).expect("entry");
    let verified = verify(raw, &sources, entry, linear, linux).expect("atomic VecPush transfer");
    let function = verified.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let failure = block
        .instructions()
        .next()
        .expect("VecPush")
        .derived_drop_actions()
        .map(|action| action.root().index())
        .collect::<Vec<_>>();
    let success = block
        .terminator()
        .derived_drop_actions()
        .map(|action| action.root().index())
        .collect::<Vec<_>>();
    assert_eq!(failure, [1, 0]);
    assert_eq!(success, [0]);
}

#[test]
#[allow(clippy::too_many_lines)]
fn direct_call_cleanup_is_after_by_value_argument_transfer() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    let caller = &mut raw.modules[0].functions[0];
    caller.entry_export = None;
    caller.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(2), span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(2), span },
        raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(1), span },
    ];
    caller.result = raw::TypeId(1);
    caller.places = vec![
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
            kind: raw::PlaceKind::Parameter(1),
        },
    ];
    caller.blocks[0].instructions = vec![raw::Instruction {
        result: Some(raw::ValueDefinition { id: raw::ValueId(3), ty: raw::TypeId(1), span }),
        span,
        kind: raw::InstructionKind::DirectCall {
            callee: raw::FunctionId { module: raw::ModuleId(0), declaration: 1 },
            arguments: vec![
                raw::CallArgument::Value(raw::ValueId(0)),
                raw::CallArgument::Value(raw::ValueId(2)),
            ],
            cleanup: raw::CleanupPlanId(0),
        },
    }];
    caller.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(3), cleanup: raw::CleanupPlanId(1) };
    caller.cleanup_plans = vec![
        raw::CleanupPlan {
            id: raw::CleanupPlanId(0),
            span,
            actions: vec![raw::DropAction::DropPlace(raw::PlaceId(1))],
        },
        raw::CleanupPlan {
            id: raw::CleanupPlanId(1),
            span,
            actions: vec![raw::DropAction::DropPlace(raw::PlaceId(1))],
        },
    ];
    raw.modules[0].functions.push(raw::Function {
        id: raw::FunctionId { module: raw::ModuleId(0), declaration: 1 },
        entry_export: None,
        span,
        parameters: vec![
            raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(2), span },
            raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span },
        ],
        borrow_parameters: vec![],
        result: raw::TypeId(1),
        places: vec![raw::Place {
            id: raw::PlaceId(0),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Parameter(0),
        }],
        blocks: vec![raw::Block {
            id: raw::BlockId(0),
            parameters: vec![],
            instructions: vec![],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Return {
                    value: raw::ValueId(1),
                    cleanup: raw::CleanupPlanId(0),
                },
            }],
        }],
        cleanup_plans: vec![raw::CleanupPlan {
            id: raw::CleanupPlanId(0),
            span,
            actions: vec![raw::DropAction::DropPlace(raw::PlaceId(0))],
        }],
    });
    let entry = sources.verify_file_id(0).expect("entry");
    let verified = verify(raw, &sources, entry, linear, linux).expect("post-transfer call cleanup");
    let roots = verified
        .modules()
        .next()
        .expect("module")
        .functions()
        .next()
        .expect("caller")
        .blocks()
        .next()
        .expect("block")
        .instructions()
        .next()
        .expect("call")
        .derived_drop_actions()
        .map(|action| action.root().index())
        .collect::<Vec<_>>();
    assert_eq!(roots, [1]);
}

#[test]
fn direct_call_rejects_foreign_identity_arity_value_and_result_types() {
    let (sources, linear, linux) = authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let mut mutations = Vec::new();

    let mut foreign_module = owned_direct_call_program(&sources, &linear, &linux);
    let raw::InstructionKind::DirectCall { callee, .. } =
        &mut foreign_module.modules[0].functions[0].blocks[0].instructions[0].kind
    else {
        panic!("call fixture")
    };
    callee.module = raw::ModuleId(99);
    mutations.push(foreign_module);

    let mut missing_function = owned_direct_call_program(&sources, &linear, &linux);
    let raw::InstructionKind::DirectCall { callee, .. } =
        &mut missing_function.modules[0].functions[0].blocks[0].instructions[0].kind
    else {
        panic!("call fixture")
    };
    callee.declaration = 99;
    mutations.push(missing_function);

    let mut wrong_arity = owned_direct_call_program(&sources, &linear, &linux);
    let raw::InstructionKind::DirectCall { arguments, .. } =
        &mut wrong_arity.modules[0].functions[0].blocks[0].instructions[0].kind
    else {
        panic!("call fixture")
    };
    arguments.pop();
    mutations.push(wrong_arity);

    let mut wrong_value = owned_direct_call_program(&sources, &linear, &linux);
    let raw::InstructionKind::DirectCall { arguments, .. } =
        &mut wrong_value.modules[0].functions[0].blocks[0].instructions[0].kind
    else {
        panic!("call fixture")
    };
    arguments[2] = raw::CallArgument::Value(raw::ValueId(2));
    let caller = &mut wrong_value.modules[0].functions[0];
    for plan in &mut caller.cleanup_plans {
        plan.actions = vec![raw::DropAction::DropPlace(raw::PlaceId(3))];
    }
    mutations.push(wrong_value);

    let mut wrong_result = owned_direct_call_program(&sources, &linear, &linux);
    let caller = &mut wrong_result.modules[0].functions[0];
    caller.blocks[0].instructions[0].result.as_mut().expect("call result").ty = raw::TypeId(0);
    caller.result = raw::TypeId(0);
    mutations.push(wrong_result);

    for mutation in mutations {
        let diagnostics = verify(mutation, &sources, entry, linear.clone(), linux.clone())
            .expect_err("invalid call signature");
        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3009"),
            "missing call diagnostic: {diagnostics:?}"
        );
    }
}

#[test]
fn direct_call_cleanup_and_callee_parameter_cleanup_are_exact() {
    let (sources, linear, linux) = authorities();
    let entry = sources.verify_file_id(0).expect("entry");

    let mut transferred = owned_direct_call_program(&sources, &linear, &linux);
    transferred.modules[0].functions[0].cleanup_plans[0]
        .actions
        .insert(0, raw::DropAction::DropPlace(raw::PlaceId(0)));
    let diagnostics = verify(transferred, &sources, entry, linear.clone(), linux.clone())
        .expect_err("transferred argument in caller cleanup");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3012"));

    let mut wrong_order = owned_direct_call_program(&sources, &linear, &linux);
    wrong_order.modules[0].functions[0].cleanup_plans[0].actions.swap(0, 1);
    let diagnostics = verify(wrong_order, &sources, entry, linear.clone(), linux.clone())
        .expect_err("caller survivor cleanup order");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3012"));

    let mut reused = owned_direct_call_program(&sources, &linear, &linux);
    let raw::Terminator::Return { cleanup, .. } =
        &mut reused.modules[0].functions[0].blocks[0].terminators[0].kind
    else {
        panic!("return fixture")
    };
    *cleanup = raw::CleanupPlanId(0);
    let diagnostics = verify(reused, &sources, entry, linear.clone(), linux.clone())
        .expect_err("call cleanup reused by return");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3012"));

    for mutate in [0_u8, 1] {
        let mut callee_cleanup = owned_direct_call_program(&sources, &linear, &linux);
        let actions = &mut callee_cleanup.modules[0].functions[1].cleanup_plans[0].actions;
        if mutate == 0 {
            actions.pop();
        } else {
            actions.swap(0, 1);
        }
        let diagnostics = verify(callee_cleanup, &sources, entry, linear.clone(), linux.clone())
            .expect_err("callee parameter cleanup");
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3012"));
    }
}

#[test]
fn mutual_call_cycle_and_static_depth_boundary_are_rejected_exactly() {
    let (sources, linear, linux) = authorities();
    let entry = sources.verify_file_id(0).expect("entry");

    let exact = scalar_call_chain(&sources, &linear, &linux, MAX_STATIC_CALL_DEPTH);
    verify(exact, &sources, entry, linear.clone(), linux.clone()).expect("exact call depth");

    let plus_one = scalar_call_chain(&sources, &linear, &linux, MAX_STATIC_CALL_DEPTH + 1);
    let diagnostics = verify(plus_one, &sources, entry, linear.clone(), linux.clone())
        .expect_err("call depth plus one");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3009"));

    let mut mutual = scalar_call_chain(&sources, &linear, &linux, 2);
    let span = mutual.modules[0].functions[1].span;
    let callee = &mut mutual.modules[0].functions[1];
    callee.blocks[0].instructions = vec![raw::Instruction {
        result: Some(raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span }),
        span,
        kind: raw::InstructionKind::DirectCall {
            callee: raw::FunctionId { module: raw::ModuleId(0), declaration: 0 },
            arguments: vec![raw::CallArgument::Value(raw::ValueId(0))],
            cleanup: raw::CleanupPlanId(0),
        },
    }];
    callee.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(1), cleanup: raw::CleanupPlanId(1) };
    callee.cleanup_plans.push(raw::CleanupPlan {
        id: raw::CleanupPlanId(1),
        span,
        actions: vec![],
    });
    let diagnostics = verify(mutual, &sources, entry, linear, linux).expect_err("mutual cycle");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3009"));
}

#[test]
fn copy_value_may_have_addressable_parameter_storage_without_an_owner_obligation() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    raw.modules[0].functions[0].places = vec![raw::Place {
        id: raw::PlaceId(0),
        ty: raw::TypeId(1),
        span,
        kind: raw::PlaceKind::Parameter(0),
    }];
    let entry = sources.verify_file_id(0).expect("entry");
    let verified = verify(raw, &sources, entry, linear, linux).expect("Copy storage place");
    let function = verified.modules().next().expect("module").functions().next().expect("function");
    let place = function.places().next().expect("parameter place");
    assert!(place.is_copy());
    assert!(matches!(place.kind(), super::VerifiedPlaceKind::Parameter(0)));
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
            cleanup: raw::CleanupPlanId(0),
        },
    });
    if let raw::Terminator::Return { value, .. } = &mut function.blocks[0].terminators[0].kind {
        *value = raw::ValueId(1);
    }
    if let raw::Terminator::Return { cleanup, .. } = &mut function.blocks[0].terminators[0].kind {
        *cleanup = raw::CleanupPlanId(1);
    }
    function.cleanup_plans.push(raw::CleanupPlan {
        id: raw::CleanupPlanId(1),
        span,
        actions: vec![],
    });
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
fn dense_shared_borrow_read_and_end_is_accepted() {
    let (sources, linear, linux) = authorities();
    let raw = shared_borrow_read_program(&sources, &linear, &linux);
    let entry = sources.verify_file_id(0).expect("entry");
    verify(raw, &sources, entry, linear, linux).expect("borrow-balanced program");
}

#[test]
fn unused_borrow_parameter_authority_is_rejected() {
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
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("unused borrow");
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code() == "ZRYNA-I3011"
            && diagnostic.message().contains("borrow parameter is not used")
    }));
}

#[test]
fn sparse_and_duplicate_borrow_parameter_metadata_is_rejected() {
    let (sources, linear, linux) = authorities();
    let span = program(&sources, &linear, &linux).modules[0].functions[0].span;
    let make = |parameters: Vec<raw::BorrowParameter>, borrow: raw::BorrowId| {
        let mut raw = program(&sources, &linear, &linux);
        let function = &mut raw.modules[0].functions[0];
        function.entry_export = None;
        function.borrow_parameters = parameters;
        function.blocks[0].instructions = vec![raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span }),
            span,
            kind: raw::InstructionKind::BorrowRead { borrow },
        }];
        function.blocks[0].terminators[0].kind =
            raw::Terminator::Return { value: raw::ValueId(1), cleanup: raw::CleanupPlanId(0) };
        raw
    };
    let parameter = |id| raw::BorrowParameter {
        id: raw::BorrowId(id),
        referent: raw::TypeId(1),
        access: raw::BorrowAccess::Shared,
        span,
    };
    let entry = sources.verify_file_id(0).expect("entry");
    for raw in [
        make(vec![parameter(1)], raw::BorrowId(1)),
        make(vec![parameter(0), parameter(0)], raw::BorrowId(0)),
    ] {
        let diagnostics = verify(raw, &sources, entry, linear.clone(), linux.clone())
            .expect_err("noncanonical borrow parameters");
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3011"));
    }
}

#[test]
fn sparse_duplicate_and_inactive_lexical_borrow_authority_is_rejected() {
    let (sources, linear, linux) = authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let seed = shared_borrow_read_program(&sources, &linear, &linux);

    let mut sparse = seed.clone();
    if let raw::InstructionKind::BeginBorrow(definition) =
        &mut sparse.modules[0].functions[0].blocks[0].instructions[1].kind
    {
        definition.id = raw::BorrowId(1);
    }
    for instruction in &mut sparse.modules[0].functions[0].blocks[0].instructions[2..] {
        match &mut instruction.kind {
            raw::InstructionKind::BorrowRead { borrow }
            | raw::InstructionKind::EndBorrow { borrow } => *borrow = raw::BorrowId(1),
            _ => {}
        }
    }
    let diagnostics = verify(sparse, &sources, entry, linear.clone(), linux.clone())
        .expect_err("sparse lexical borrow");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3011"));

    let mut duplicate = seed.clone();
    let begin = duplicate.modules[0].functions[0].blocks[0].instructions[1].clone();
    duplicate.modules[0].functions[0].blocks[0].instructions.insert(2, begin);
    let diagnostics = verify(duplicate, &sources, entry, linear.clone(), linux.clone())
        .expect_err("duplicate lexical borrow");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3011"));

    let mut inactive = seed;
    inactive.modules[0].functions[0].blocks[0].instructions.swap(2, 3);
    let diagnostics =
        verify(inactive, &sources, entry, linear, linux).expect_err("read after lexical end");
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code() == "ZRYNA-I3011"
            && diagnostic.message().contains("borrow read uses an inactive authority")
    }));
}

#[test]
fn lexical_borrow_rejects_uninitialized_and_moved_roots() {
    let (sources, linear, linux) = authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let seed = shared_borrow_read_program(&sources, &linear, &linux);

    let mut uninitialized = seed.clone();
    uninitialized.modules[0].functions[0].blocks[0].instructions.remove(0);
    let diagnostics = verify(uninitialized, &sources, entry, linear.clone(), linux.clone())
        .expect_err("borrow of uninitialized root");
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code() == "ZRYNA-I3011"
            && diagnostic.message().contains("borrow identity, owner state, or overlap is invalid")
    }));

    let mut moved = string_literal_program(&sources, &linear, &linux, b"root".to_vec());
    let function = &mut moved.modules[0].functions[0];
    let span = function.span;
    function.places = vec![
        raw::Place {
            id: raw::PlaceId(0),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(0)),
        },
        raw::Place {
            id: raw::PlaceId(1),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Local(0),
        },
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
            kind: raw::PlaceKind::Local(1),
        },
        raw::Place {
            id: raw::PlaceId(4),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(2)),
        },
    ];
    function.blocks[0].instructions.extend([
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::InitializePlace {
                place: raw::PlaceId(1),
                value: raw::ValueId(0),
            },
        },
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(2), span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(1) },
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
            kind: raw::InstructionKind::BeginBorrow(raw::BorrowDefinition {
                id: raw::BorrowId(0),
                place: raw::PlaceId(1),
                access: raw::BorrowAccess::Shared,
                span,
            }),
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::EndBorrow { borrow: raw::BorrowId(0) },
        },
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(2), span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(3) },
        },
    ]);
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(2), cleanup: raw::CleanupPlanId(1) };
    let diagnostics =
        verify(moved, &sources, entry, linear, linux).expect_err("borrow of moved root");
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code() == "ZRYNA-I3011"
            && diagnostic.message().contains("borrow identity, owner state, or overlap is invalid")
    }));
}

#[test]
fn lexical_borrow_rejects_double_end_with_inactive_authority_diagnostic() {
    let (sources, linear, linux) = authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let mut raw = shared_borrow_read_program(&sources, &linear, &linux);
    let duplicate_end = raw.modules[0].functions[0].blocks[0].instructions[3].clone();
    raw.modules[0].functions[0].blocks[0].instructions.push(duplicate_end);
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("double borrow end");
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code() == "ZRYNA-I3011"
            && diagnostic.message().contains("borrow end uses an inactive authority")
    }));
}

#[test]
fn lexical_borrow_cannot_escape_return_or_trap() {
    let (sources, linear, linux) = authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let mut returned = shared_borrow_read_program(&sources, &linear, &linux);
    returned.modules[0].functions[0].blocks[0].instructions.pop();
    let diagnostics = verify(returned.clone(), &sources, entry, linear.clone(), linux.clone())
        .expect_err("borrow crossing return");
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code() == "ZRYNA-I3011"
            && diagnostic.message().contains("borrow remains active at a control-flow edge")
    }));

    returned.modules[0].functions[0].blocks[0].terminators[0].kind = raw::Terminator::Trap {
        identity: raw::TrapIdentity::BoundsV1,
        cleanup: raw::CleanupPlanId(0),
    };
    let diagnostics =
        verify(returned, &sources, entry, linear, linux).expect_err("borrow crossing trap");
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code() == "ZRYNA-I3011"
            && diagnostic.message().contains("borrow remains active at a control-flow edge")
    }));
}

#[test]
fn borrow_parameter_cannot_be_ended_or_exported() {
    let (sources, linear, linux) = authorities();
    let span = program(&sources, &linear, &linux).modules[0].functions[0].span;
    let mut raw = program(&sources, &linear, &linux);
    let function = &mut raw.modules[0].functions[0];
    function.entry_export = None;
    function.borrow_parameters = vec![raw::BorrowParameter {
        id: raw::BorrowId(0),
        referent: raw::TypeId(1),
        access: raw::BorrowAccess::Shared,
        span,
    }];
    function.blocks[0].instructions = vec![
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span }),
            span,
            kind: raw::InstructionKind::BorrowRead { borrow: raw::BorrowId(0) },
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
    let diagnostics = verify(raw.clone(), &sources, entry, linear.clone(), linux.clone())
        .expect_err("callee cannot end parameter authority");
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code() == "ZRYNA-I3011"
            && diagnostic.message().contains("borrow end uses an inactive authority")
    }));

    raw.modules[0].functions[0].entry_export = Some("id".into());
    raw.modules[0].functions[0].blocks[0].instructions.pop();
    let diagnostics =
        verify(raw, &sources, entry, linear, linux).expect_err("borrowed public export");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3009"));
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
    function.result = raw::TypeId(1);
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(1), cleanup: raw::CleanupPlanId(0) };
    function.cleanup_plans[0].actions = vec![
        raw::DropAction::DropPlace(raw::PlaceId(2)),
        raw::DropAction::DropPlace(raw::PlaceId(0)),
    ];
    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("payload dominance");
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3013"),
        "{diagnostics:?}"
    );
}

#[test]
#[allow(clippy::too_many_lines)]
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
        raw::Place {
            id: raw::PlaceId(3),
            ty: raw::TypeId(3),
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(3)),
        },
        raw::Place {
            id: raw::PlaceId(4),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(3), ordinal: 0 },
        },
        raw::Place {
            id: raw::PlaceId(5),
            ty: raw::TypeId(3),
            span,
            kind: raw::PlaceKind::Local(0),
        },
        raw::Place {
            id: raw::PlaceId(6),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(5), ordinal: 0 },
        },
    ];
    function.blocks[0].instructions.extend([
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(2), span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(1) },
        },
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(3), ty: raw::TypeId(3), span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(0) },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::InitializePlace {
                place: raw::PlaceId(5),
                value: raw::ValueId(3),
            },
        },
    ]);
    if let raw::Terminator::Return { value, .. } = &mut function.blocks[0].terminators[0].kind {
        *value = raw::ValueId(1);
    }
    function.cleanup_plans[0].actions = vec![
        raw::DropAction::DropPlace(raw::PlaceId(2)),
        raw::DropAction::DropPlace(raw::PlaceId(5)),
    ];
    let entry = sources.verify_file_id(0).expect("entry");
    let verified = verify(raw.clone(), &sources, entry, linear.clone(), linux.clone())
        .expect("partial cleanup remains complete after rename");
    let actions = verified
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
        .collect::<Vec<_>>();
    assert_eq!(
        actions[1].moved_projections().map(super::PlaceIdentity::index).collect::<Vec<_>>(),
        [6]
    );

    let mut missing_destination = raw.clone();
    missing_destination.modules[0].functions[0].places.remove(4);
    for (index, place) in
        missing_destination.modules[0].functions[0].places.iter_mut().enumerate().skip(4)
    {
        place.id = raw::PlaceId(u32::try_from(index).expect("dense place id"));
    }
    if let raw::PlaceKind::Local(_) = missing_destination.modules[0].functions[0].places[4].kind {
    } else {
        panic!("renumbered local root")
    }
    if let raw::PlaceKind::StructField { base, .. } =
        &mut missing_destination.modules[0].functions[0].places[5].kind
    {
        *base = raw::PlaceId(4);
    }
    if let raw::InstructionKind::InitializePlace { place, .. } =
        &mut missing_destination.modules[0].functions[0].blocks[0].instructions[2].kind
    {
        *place = raw::PlaceId(4);
    }
    missing_destination.modules[0].functions[0].cleanup_plans[0].actions[1] =
        raw::DropAction::DropPlace(raw::PlaceId(4));
    let diagnostics = verify(missing_destination, &sources, entry, linear.clone(), linux.clone())
        .expect_err("partial move rename without matching destination projection metadata");
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code() == "ZRYNA-I3010"
            && diagnostic
                .message()
                .contains("partial owner rename requires exact matching projection metadata")
    }));

    let mut missing_local_projection = raw;
    missing_local_projection.modules[0].functions[0].places.pop();
    let diagnostics = verify(missing_local_projection, &sources, entry, linear, linux)
        .expect_err("partial local initialization without matching destination metadata");
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code() == "ZRYNA-I3010"
            && diagnostic
                .message()
                .contains("partial owner rename requires exact matching projection metadata")
    }));
}

#[test]
fn aggregate_subobject_move_renames_into_an_exact_local_with_masked_parent_cleanup() {
    let (sources, linear, linux, inner, outer, array) = subobject_move_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    for shape in [SubobjectMoveShape::Struct, SubobjectMoveShape::FixedArray] {
        let raw =
            aggregate_subobject_move_program(&sources, &linear, &linux, inner, outer, array, shape);
        let verified = verify(raw, &sources, entry, linear.clone(), linux.clone())
            .expect("aggregate subobject move into exact local");
        let function =
            verified.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        assert_eq!(
            block.instructions().map(super::VerifiedInstruction::kind).collect::<Vec<_>>(),
            [VerifiedInstructionKind::MoveFromPlace, VerifiedInstructionKind::InitializePlace]
        );
        let cleanup = block.terminator().derived_drop_actions().collect::<Vec<_>>();
        assert_eq!(cleanup.iter().map(|action| action.root().index()).collect::<Vec<_>>(), [5, 0]);
        assert_eq!(cleanup[0].moved_projections().count(), 0);
        assert_eq!(
            cleanup[1].moved_projections().map(super::PlaceIdentity::index).collect::<Vec<_>>(),
            [1, 2]
        );
    }
}

#[test]
fn projected_aggregate_clone_is_sealed_to_one_immediate_direct_local() {
    let (sources, linear, linux, inner, outer, array) = subobject_move_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    for shape in [SubobjectMoveShape::Struct, SubobjectMoveShape::FixedArray] {
        let raw = projected_aggregate_clone_program(
            &sources, &linear, &linux, inner, outer, array, shape,
        );
        let verified = verify(raw, &sources, entry, linear.clone(), linux.clone())
            .expect("projected aggregate clone direct local");
        let function =
            verified.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(instructions.len(), 2);
        let clone = instructions[0];
        assert_eq!(clone.kind(), VerifiedInstructionKind::ClonePlace);
        assert_eq!(clone.place_operands().next().expect("source").index(), 1);
        assert_eq!(clone.result().expect("result").index(), 2);
        assert_eq!(clone.aggregate_clone_fallible_leaf_count(), Some(1));
        assert_eq!(
            clone.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
            [0],
        );
        assert_eq!(
            clone
                .aggregate_clone_element_failure_drop_actions()
                .map(|action| (action.kind(), action.root().index()))
                .collect::<Vec<_>>(),
            [
                (VerifiedDropActionKind::AggregateInitializedPrefix, 2),
                (VerifiedDropActionKind::Place, 0),
            ],
        );
        assert_eq!(instructions[1].kind(), VerifiedInstructionKind::InitializePlace);
        assert_eq!(instructions[1].place_operands().next().expect("local").index(), 3);
        assert_eq!(instructions[1].value_operands().next().expect("clone value").index(), 2);
    }
}

#[test]
fn projected_aggregate_clone_rejects_forged_contexts_and_second_sites() {
    let (sources, linear, linux, inner, outer, array) = subobject_move_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let baseline = projected_aggregate_clone_program(
        &sources,
        &linear,
        &linux,
        inner,
        outer,
        array,
        SubobjectMoveShape::Struct,
    );
    let mut mutations = Vec::new();

    let mut missing_local = baseline.clone();
    missing_local.modules[0].functions[0].blocks[0].instructions.pop();
    mutations.push(("missing local", "ZRYNA-I3010", missing_local));

    let mut public = baseline.clone();
    public.modules[0].functions[0].entry_export = Some("forged".into());
    mutations.push(("public", "ZRYNA-I3010", public));

    let mut wrong_target = baseline.clone();
    let raw::InstructionKind::InitializePlace { place, .. } =
        &mut wrong_target.modules[0].functions[0].blocks[0].instructions[1].kind
    else {
        panic!("initialize")
    };
    *place = raw::PlaceId(2);
    mutations.push(("temporary target", "ZRYNA-I3010", wrong_target));

    let mut enum_payload = baseline.clone();
    enum_payload.modules[0].functions[0].places[1].kind =
        raw::PlaceKind::EnumPayload { base: raw::PlaceId(0), variant: 0 };
    mutations.push(("enum payload", "ZRYNA-I3006", enum_payload));

    let mut second = baseline.clone();
    let function = &mut second.modules[0].functions[0];
    let span = function.span;
    function.places.extend([
        raw::Place {
            id: raw::PlaceId(4),
            ty: inner,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(3)),
        },
        raw::Place { id: raw::PlaceId(5), ty: inner, span, kind: raw::PlaceKind::Local(1) },
    ]);
    function.blocks[0].instructions.extend([
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(3), ty: inner, span }),
            span,
            kind: raw::InstructionKind::ClonePlace {
                place: raw::PlaceId(1),
                cleanup: raw::CleanupPlanId(3),
                element_cleanup: Some(raw::CleanupPlanId(4)),
            },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::InitializePlace {
                place: raw::PlaceId(5),
                value: raw::ValueId(3),
            },
        },
    ]);
    function.cleanup_plans.extend([
        raw::CleanupPlan {
            id: raw::CleanupPlanId(3),
            span,
            actions: vec![
                raw::DropAction::DropPlace(raw::PlaceId(3)),
                raw::DropAction::DropPlace(raw::PlaceId(0)),
            ],
        },
        raw::CleanupPlan {
            id: raw::CleanupPlanId(4),
            span,
            actions: vec![
                raw::DropAction::DropAggregateInitializedPrefix(raw::PlaceId(4)),
                raw::DropAction::DropPlace(raw::PlaceId(3)),
                raw::DropAction::DropPlace(raw::PlaceId(0)),
            ],
        },
    ]);
    function.cleanup_plans[2].actions.insert(0, raw::DropAction::DropPlace(raw::PlaceId(5)));
    mutations.push(("second site", "ZRYNA-I3010", second));

    for (label, code, mutation) in mutations {
        let diagnostics =
            verify(mutation, &sources, entry, linear.clone(), linux.clone()).expect_err(label);
        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic.code() == code),
            "{label}: {diagnostics:?}",
        );
    }
}

#[test]
fn projected_aggregate_assignment_is_sealed_to_one_root_to_static_projection_move() {
    let (sources, linear, linux, inner, outer, array) = subobject_move_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    for shape in [SubobjectMoveShape::Struct, SubobjectMoveShape::FixedArray] {
        let raw = projected_aggregate_assignment_program(
            &sources, &linear, &linux, inner, outer, array, shape,
        );
        let verified = verify(raw, &sources, entry, linear.clone(), linux.clone())
            .expect("projected aggregate assignment");
        let function =
            verified.modules().next().expect("module").functions().next().expect("function");
        let instructions =
            function.blocks().next().expect("block").instructions().collect::<Vec<_>>();
        assert_eq!(instructions[2].kind(), VerifiedInstructionKind::MoveFromPlace);
        assert_eq!(instructions[2].place_operands().next().expect("source").index(), 4);
        assert_eq!(instructions[3].kind(), VerifiedInstructionKind::ReplacePlace);
        assert_eq!(instructions[3].place_operands().next().expect("target").index(), 3);
        assert_eq!(instructions[3].value_operands().next(), instructions[2].result());
        assert_eq!(
            instructions[3]
                .derived_drop_actions()
                .map(|action| action.root().index())
                .collect::<Vec<_>>(),
            [3],
        );
    }
}

#[test]
fn projected_subobject_assignment_moves_one_complete_static_subobject() {
    let (sources, linear, linux, inner, outer, array) = subobject_move_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    for shape in [SubobjectMoveShape::Struct, SubobjectMoveShape::FixedArray] {
        let raw = projected_subobject_assignment_program(
            &sources, &linear, &linux, inner, outer, array, shape,
        );
        let raw_function = &raw.modules[0].functions[0];
        let move_index = raw_function.blocks[0]
            .instructions
            .iter()
            .position(|instruction| {
                matches!(instruction.kind, raw::InstructionKind::MoveFromPlace { .. })
            })
            .expect("projected move");
        let raw::InstructionKind::MoveFromPlace { place: source } =
            raw_function.blocks[0].instructions[move_index].kind
        else {
            unreachable!()
        };
        let raw::InstructionKind::ReplacePlace { place: target, .. } =
            raw_function.blocks[0].instructions[move_index + 1].kind
        else {
            unreachable!()
        };
        let source_root = super::projection_base(&raw_function.places[source.0 as usize].kind)
            .expect("source root");
        let target_root = super::projection_base(&raw_function.places[target.0 as usize].kind)
            .expect("target root");
        let verified = verify(raw, &sources, entry, linear.clone(), linux.clone())
            .expect("projected subobject assignment");
        let function =
            verified.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(instructions[move_index].kind(), VerifiedInstructionKind::MoveFromPlace);
        assert_eq!(
            instructions[move_index].place_operands().next().expect("source").index(),
            source.0
        );
        assert_eq!(instructions[move_index + 1].kind(), VerifiedInstructionKind::ReplacePlace);
        assert_eq!(
            instructions[move_index + 1].place_operands().next().expect("target").index(),
            target.0
        );
        assert_eq!(
            instructions[move_index + 1].value_operands().next(),
            instructions[move_index].result()
        );
        assert_eq!(
            instructions[move_index + 1]
                .derived_drop_actions()
                .map(|action| action.root().index())
                .collect::<Vec<_>>(),
            [target.0],
        );
        let cleanup = block.terminator().derived_drop_actions().collect::<Vec<_>>();
        assert_eq!(
            cleanup.iter().map(|action| action.root().index()).collect::<Vec<_>>(),
            [source_root.0, target_root.0]
        );
        assert_eq!(
            cleanup[0].moved_projections().map(super::PlaceIdentity::index).collect::<Vec<_>>(),
            [source.0, source.0 + 1],
        );
        assert_eq!(cleanup[1].moved_projections().count(), 0);
    }
}

#[test]
fn projected_subobject_assignment_clones_one_static_subobject_without_descendant_places() {
    let (sources, linear, linux, inner, outer, array) = subobject_move_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    for shape in [SubobjectMoveShape::Struct, SubobjectMoveShape::FixedArray] {
        let raw = projected_subobject_clone_assignment_program(
            &sources, &linear, &linux, inner, outer, array, shape,
        );
        let raw_function = &raw.modules[0].functions[0];
        let clone_index = raw_function.blocks[0]
            .instructions
            .iter()
            .position(|instruction| {
                matches!(instruction.kind, raw::InstructionKind::ClonePlace { .. })
            })
            .expect("projected clone");
        let raw::InstructionKind::ClonePlace { place: source, .. } =
            raw_function.blocks[0].instructions[clone_index].kind
        else {
            unreachable!()
        };
        let raw::InstructionKind::ReplacePlace { place: target, .. } =
            raw_function.blocks[0].instructions[clone_index + 1].kind
        else {
            unreachable!()
        };
        let source_root = super::projection_base(&raw_function.places[source.0 as usize].kind)
            .expect("source root");
        let target_root = super::projection_base(&raw_function.places[target.0 as usize].kind)
            .expect("target root");
        assert!(
            raw_function
                .places
                .iter()
                .all(|place| super::projection_base(&place.kind) != Some(source))
        );

        let verified = verify(raw, &sources, entry, linear.clone(), linux.clone())
            .expect("projected subobject clone assignment");
        let function =
            verified.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        let clone = instructions[clone_index];
        assert_eq!(clone.kind(), VerifiedInstructionKind::ClonePlace);
        assert_eq!(clone.place_operands().next().expect("source").index(), source.0);
        assert_eq!(clone.aggregate_clone_fallible_leaf_count(), Some(1));
        assert_eq!(
            clone.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
            [source_root.0, target_root.0],
        );
        let prefix_cleanup =
            clone.aggregate_clone_element_failure_drop_actions().collect::<Vec<_>>();
        assert_eq!(prefix_cleanup[0].kind(), VerifiedDropActionKind::AggregateInitializedPrefix);
        assert_eq!(
            prefix_cleanup[1..].iter().map(|action| action.root().index()).collect::<Vec<_>>(),
            [source_root.0, target_root.0],
        );
        let replace = instructions[clone_index + 1];
        assert_eq!(replace.kind(), VerifiedInstructionKind::ReplacePlace);
        assert_eq!(replace.place_operands().next().expect("target").index(), target.0);
        assert_eq!(replace.value_operands().next(), clone.result());
        assert_eq!(
            replace.derived_drop_actions().map(|action| action.root().index()).collect::<Vec<_>>(),
            [target.0],
        );
        let cleanup = block.terminator().derived_drop_actions().collect::<Vec<_>>();
        assert_eq!(
            cleanup.iter().map(|action| action.root().index()).collect::<Vec<_>>(),
            [source_root.0, target_root.0],
        );
        assert!(cleanup.iter().all(|action| action.moved_projections().count() == 0));
    }
}

#[test]
fn projected_subobject_clone_assignment_rejects_overlap_order_and_alternate_use() {
    let (sources, linear, linux, inner, outer, array) = subobject_move_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let baseline = projected_subobject_clone_assignment_program(
        &sources,
        &linear,
        &linux,
        inner,
        outer,
        array,
        SubobjectMoveShape::FixedArray,
    );
    let function = &baseline.modules[0].functions[0];
    let span = function.span;
    let clone_index = function.blocks[0]
        .instructions
        .iter()
        .position(|instruction| matches!(instruction.kind, raw::InstructionKind::ClonePlace { .. }))
        .expect("projected clone");
    let raw::InstructionKind::ClonePlace { place: source, .. } =
        function.blocks[0].instructions[clone_index].kind
    else {
        unreachable!()
    };
    let result = function.blocks[0].instructions[clone_index].result.expect("clone result");
    let raw::InstructionKind::ReplacePlace { place: target, .. } =
        function.blocks[0].instructions[clone_index + 1].kind
    else {
        unreachable!()
    };
    let target_root =
        super::projection_base(&function.places[target.0 as usize].kind).expect("target root");
    let result_temporary = function
        .places
        .iter()
        .find(|place| matches!(place.kind, raw::PlaceKind::Temporary(owner) if owner == result.id))
        .expect("clone temporary")
        .id;
    let mut mutations = Vec::new();

    let mut same_root = baseline.clone();
    same_root.modules[0].functions[0].places[source.0 as usize].kind =
        raw::PlaceKind::FixedArrayConstant { base: target_root, index: 1 };
    mutations.push(("same root", same_root));

    let mut nonadjacent = baseline.clone();
    nonadjacent.modules[0].functions[0].blocks[0].instructions.insert(
        clone_index + 1,
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::DropPlace { place: target_root },
        },
    );
    mutations.push(("nonadjacent replacement", nonadjacent));

    let mut alternate_use = baseline;
    alternate_use.modules[0].functions[0].blocks[0].instructions.push(raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::InitializePlace { place: result_temporary, value: result.id },
    });
    mutations.push(("alternate clone result use", alternate_use));

    for (label, raw) in mutations {
        let diagnostics =
            verify(raw, &sources, entry, linear.clone(), linux.clone()).expect_err(label);
        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"),
            "{label}: {diagnostics:?}",
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn projected_subobject_assignment_rejects_hostile_contexts() {
    let (sources, linear, linux, inner, outer, array) = subobject_move_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let baseline = projected_subobject_assignment_program(
        &sources,
        &linear,
        &linux,
        inner,
        outer,
        array,
        SubobjectMoveShape::FixedArray,
    );
    let span = baseline.modules[0].functions[0].span;
    let baseline_function = &baseline.modules[0].functions[0];
    let move_index = baseline_function.blocks[0]
        .instructions
        .iter()
        .position(|instruction| {
            matches!(instruction.kind, raw::InstructionKind::MoveFromPlace { .. })
        })
        .expect("projected move");
    let raw::InstructionKind::MoveFromPlace { place: source } =
        baseline_function.blocks[0].instructions[move_index].kind
    else {
        unreachable!()
    };
    let moved =
        baseline_function.blocks[0].instructions[move_index].result.expect("moved value").id;
    let raw::InstructionKind::ReplacePlace { place: target, .. } =
        baseline_function.blocks[0].instructions[move_index + 1].kind
    else {
        unreachable!()
    };
    let source_root = super::projection_base(&baseline_function.places[source.0 as usize].kind)
        .expect("source root");
    let target_root = super::projection_base(&baseline_function.places[target.0 as usize].kind)
        .expect("target root");
    let source_leaf = baseline_function
        .places
        .iter()
        .find(|place| super::projection_base(&place.kind) == Some(source))
        .expect("source leaf")
        .id;
    let moved_temporary = baseline_function
        .places
        .iter()
        .find(|place| matches!(place.kind, raw::PlaceKind::Temporary(owner) if owner == moved))
        .expect("moved temporary")
        .id;
    let mut mutations = Vec::new();

    let mut same_root = baseline.clone();
    same_root.modules[0].functions[0].places[source.0 as usize].kind =
        raw::PlaceKind::FixedArrayConstant { base: target_root, index: 1 };
    mutations.push(("same root", same_root));

    for hostile_root in
        [raw::PlaceKind::Parameter(0), raw::PlaceKind::Temporary(raw::ValueId(u32::MAX))]
    {
        let mut hostile = baseline.clone();
        hostile.modules[0].functions[0].places[source_root.0 as usize].kind = hostile_root;
        let function = &hostile.modules[0].functions[0];
        let result = function.blocks[0].instructions[move_index].result.expect("move result");
        assert!(!super::admitted_projected_assignment_source(
            source,
            result,
            false,
            target,
            Some(moved_temporary),
            function,
            &linear,
        ));
        assert!(!super::is_complete_projected_subobject_replacement(
            &function.blocks[0].instructions[move_index + 1],
            source,
            result,
            Some(moved_temporary),
            function,
        ));
    }
    assert!(super::admitted_projected_assignment_source(
        source,
        baseline_function.blocks[0].instructions[move_index].result.expect("move result"),
        true,
        target,
        Some(moved_temporary),
        baseline_function,
        &linear,
    ));

    let mut nonadjacent = baseline.clone();
    nonadjacent.modules[0].functions[0].blocks[0].instructions.insert(
        move_index + 1,
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::DropPlace { place: target_root },
        },
    );
    mutations.push(("nonadjacent replacement", nonadjacent));

    let mut alternate_use = baseline.clone();
    alternate_use.modules[0].functions[0].blocks[0].instructions.push(raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::InitializePlace { place: moved_temporary, value: moved },
    });
    mutations.push(("alternate temporary use", alternate_use));

    let mut incomplete_topology = baseline.clone();
    incomplete_topology.modules[0].functions[0].places[source_leaf.0 as usize].kind =
        raw::PlaceKind::Local(99);
    mutations.push(("incomplete source topology", incomplete_topology));

    for (label, raw) in mutations {
        let diagnostics =
            verify(raw, &sources, entry, linear.clone(), linux.clone()).expect_err(label);
        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"),
            "{label}: {diagnostics:?}",
        );
    }
}

#[test]
fn projected_aggregate_assignment_accepts_one_root_clone_into_a_static_projection() {
    let (sources, linear, linux, inner, outer, array) = subobject_move_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    for shape in [SubobjectMoveShape::Struct, SubobjectMoveShape::FixedArray] {
        let raw = projected_aggregate_clone_assignment_program(
            &sources, &linear, &linux, inner, outer, array, shape,
        );
        let verified = verify(raw, &sources, entry, linear.clone(), linux.clone())
            .expect("projected aggregate clone assignment");
        let function =
            verified.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        assert_eq!(instructions[2].kind(), VerifiedInstructionKind::ClonePlace);
        assert_eq!(instructions[2].place_operands().next().expect("source").index(), 4);
        assert_eq!(instructions[3].kind(), VerifiedInstructionKind::ReplacePlace);
        assert_eq!(instructions[3].place_operands().next().expect("target").index(), 3);
        assert_eq!(instructions[3].value_operands().next(), instructions[2].result());
        assert_eq!(
            instructions[2]
                .derived_drop_actions()
                .map(|action| action.root().index())
                .collect::<Vec<_>>(),
            [4, 2],
        );
        assert_eq!(
            instructions[3]
                .derived_drop_actions()
                .map(|action| action.root().index())
                .collect::<Vec<_>>(),
            [3],
        );
        assert_eq!(
            block
                .terminator()
                .derived_drop_actions()
                .map(|action| action.root().index())
                .collect::<Vec<_>>(),
            [4, 2],
            "successful clone retains its source",
        );
    }
}

#[test]
fn projected_aggregate_clone_assignment_rejects_forged_source_order_and_uses() {
    let (sources, linear, linux, inner, outer, array) = subobject_move_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let baseline = projected_aggregate_clone_assignment_program(
        &sources,
        &linear,
        &linux,
        inner,
        outer,
        array,
        SubobjectMoveShape::Struct,
    );
    let span = baseline.modules[0].functions[0].span;
    let mut mutations = Vec::new();

    let mut projected_source = baseline.clone();
    let raw::InstructionKind::ClonePlace { place, .. } =
        &mut projected_source.modules[0].functions[0].blocks[0].instructions[2].kind
    else {
        panic!("clone")
    };
    *place = raw::PlaceId(3);
    mutations.push(("projected source", projected_source));

    let mut nonadjacent = baseline.clone();
    nonadjacent.modules[0].functions[0].blocks[0].instructions.insert(
        3,
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(0) },
        },
    );
    mutations.push(("nonadjacent clone", nonadjacent));

    let mut alternate_use = baseline;
    alternate_use.modules[0].functions[0].blocks[0].instructions.push(raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::InitializePlace {
            place: raw::PlaceId(4),
            value: raw::ValueId(3),
        },
    });
    mutations.push(("alternate clone result use", alternate_use));

    for (label, raw) in mutations {
        let diagnostics =
            verify(raw, &sources, entry, linear.clone(), linux.clone()).expect_err(label);
        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"),
            "{label}: {diagnostics:?}",
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn projected_aggregate_assignment_rejects_forged_contexts_and_second_sites() {
    let (sources, linear, linux, inner, outer, array) = subobject_move_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let baseline = projected_aggregate_assignment_program(
        &sources,
        &linear,
        &linux,
        inner,
        outer,
        array,
        SubobjectMoveShape::Struct,
    );
    let mut mutations = Vec::new();

    let mut nonadjacent = baseline.clone();
    let span = nonadjacent.modules[0].functions[0].span;
    nonadjacent.modules[0].functions[0].blocks[0].instructions.insert(
        3,
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(0) },
        },
    );
    mutations.push(("nonadjacent move", "ZRYNA-I3010", nonadjacent));

    let mut projected_source = baseline.clone();
    let raw::InstructionKind::MoveFromPlace { place } =
        &mut projected_source.modules[0].functions[0].blocks[0].instructions[2].kind
    else {
        panic!("move")
    };
    *place = raw::PlaceId(3);
    mutations.push(("projected source", "ZRYNA-I3010", projected_source));

    let mut duplicate_temporary = baseline.clone();
    duplicate_temporary.modules[0].functions[0].places.push(raw::Place {
        id: raw::PlaceId(6),
        ty: inner,
        span,
        kind: raw::PlaceKind::Temporary(raw::ValueId(3)),
    });
    mutations.push(("duplicate temporary", "ZRYNA-I3006", duplicate_temporary));

    let mut moved_target = baseline.clone();
    let function = &mut moved_target.modules[0].functions[0];
    let raw::PlaceKind::Temporary(owner) = &mut function.places[5].kind else {
        panic!("temporary")
    };
    *owner = raw::ValueId(4);
    function.places.push(raw::Place {
        id: raw::PlaceId(6),
        ty: inner,
        span,
        kind: raw::PlaceKind::Temporary(raw::ValueId(3)),
    });
    let raw::InstructionKind::MoveFromPlace { .. } = &function.blocks[0].instructions[2].kind
    else {
        panic!("source move")
    };
    function.blocks[0].instructions[2].result =
        Some(raw::ValueDefinition { id: raw::ValueId(4), ty: inner, span });
    let raw::InstructionKind::ReplacePlace { value, .. } =
        &mut function.blocks[0].instructions[3].kind
    else {
        panic!("replacement")
    };
    *value = raw::ValueId(4);
    function.blocks[0].instructions.splice(
        2..2,
        [
            raw::Instruction {
                result: Some(raw::ValueDefinition { id: raw::ValueId(3), ty: inner, span }),
                span,
                kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(3) },
            },
            raw::Instruction {
                result: None,
                span,
                kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(6) },
            },
        ],
    );
    mutations.push(("moved target", "ZRYNA-I3010", moved_target));

    let mut second_site = baseline.clone();
    let function = &mut second_site.modules[0].functions[0];
    function.places.push(raw::Place {
        id: raw::PlaceId(6),
        ty: inner,
        span,
        kind: raw::PlaceKind::Temporary(raw::ValueId(4)),
    });
    function.blocks[0].instructions.extend([
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(4), ty: inner, span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(5) },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::ReplacePlace {
                place: raw::PlaceId(3),
                value: raw::ValueId(4),
            },
        },
    ]);
    mutations.push(("second site", "ZRYNA-I3010", second_site));

    let mut mixed_second_site = projected_aggregate_clone_assignment_program(
        &sources,
        &linear,
        &linux,
        inner,
        outer,
        array,
        SubobjectMoveShape::Struct,
    );
    let function = &mut mixed_second_site.modules[0].functions[0];
    function.places.push(raw::Place {
        id: raw::PlaceId(6),
        ty: inner,
        span,
        kind: raw::PlaceKind::Temporary(raw::ValueId(4)),
    });
    function.blocks[0].instructions.extend([
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(4), ty: inner, span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(5) },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::ReplacePlace {
                place: raw::PlaceId(3),
                value: raw::ValueId(4),
            },
        },
    ]);
    mutations.push(("mixed clone and move second site", "ZRYNA-I3010", mixed_second_site));

    for (label, expected, raw) in mutations {
        let diagnostics =
            verify(raw, &sources, entry, linear.clone(), linux.clone()).expect_err(label);
        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic.code() == expected),
            "{label}: {diagnostics:?}",
        );
    }
}

#[test]
fn aggregate_subobject_move_rejects_forged_cleanup_claims() {
    let (sources, linear, linux, inner, outer, array) = subobject_move_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let raw = aggregate_subobject_move_program(
        &sources,
        &linear,
        &linux,
        inner,
        outer,
        array,
        SubobjectMoveShape::Struct,
    );
    let cases = [
        (
            vec![
                raw::DropAction::DropPlace(raw::PlaceId(5)),
                raw::DropAction::DropPlace(raw::PlaceId(1)),
                raw::DropAction::DropPlace(raw::PlaceId(0)),
            ],
            "moved subobject listed separately",
        ),
        (vec![raw::DropAction::DropPlace(raw::PlaceId(0))], "new local omitted"),
        (
            vec![
                raw::DropAction::DropPlace(raw::PlaceId(5)),
                raw::DropAction::DropPlace(raw::PlaceId(5)),
                raw::DropAction::DropPlace(raw::PlaceId(0)),
            ],
            "new local duplicated",
        ),
    ];
    for (actions, reason) in cases {
        let mut forged = raw.clone();
        forged.modules[0].functions[0].cleanup_plans[0].actions = actions;
        let diagnostics =
            verify(forged, &sources, entry, linear.clone(), linux.clone()).expect_err(reason);
        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3012"),
            "{reason}: {diagnostics:?}"
        );
    }
}

#[test]
fn aggregate_subobject_move_rejects_forged_projection_type_and_base() {
    let (sources, linear, linux, inner, outer, array) = subobject_move_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    for shape in [SubobjectMoveShape::Struct, SubobjectMoveShape::FixedArray] {
        let raw =
            aggregate_subobject_move_program(&sources, &linear, &linux, inner, outer, array, shape);

        let mut wrong_type = raw.clone();
        wrong_type.modules[0].functions[0].places[1].ty = raw::TypeId(2);
        let diagnostics = verify(wrong_type, &sources, entry, linear.clone(), linux.clone())
            .expect_err("wrong aggregate projection type");
        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3006"),
            "{diagnostics:?}"
        );

        let mut wrong_base = raw;
        wrong_base.modules[0].functions[0].places[1].kind =
            raw::PlaceKind::StructField { base: raw::PlaceId(0), ordinal: 1 };
        let diagnostics = verify(wrong_base, &sources, entry, linear.clone(), linux.clone())
            .expect_err("wrong aggregate projection base selector");
        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3006"),
            "{diagnostics:?}"
        );
    }
}

#[test]
fn aggregate_subobject_move_is_sealed_to_one_exact_final_return() {
    let (sources, linear, linux, inner, outer, array) = subobject_move_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    for shape in [SubobjectMoveShape::Struct, SubobjectMoveShape::FixedArray] {
        let direct_return = aggregate_subobject_return_program(
            &sources, &linear, &linux, inner, outer, array, shape,
        );
        let verified = verify(direct_return, &sources, entry, linear.clone(), linux.clone())
            .expect("aggregate projection move final return");
        let function =
            verified.modules().next().expect("module").functions().next().expect("function");
        let block = function.blocks().next().expect("block");
        let instructions = block.instructions().collect::<Vec<_>>();
        let moved = instructions.last().expect("final projected move");
        assert_eq!(moved.kind(), VerifiedInstructionKind::MoveFromPlace);
        assert_eq!(block.terminator().kind(), VerifiedTerminatorKind::Return);
        assert_eq!(block.terminator().value_operands().next(), moved.result());
        let cleanup = block.terminator().derived_drop_actions().collect::<Vec<_>>();
        assert_eq!(cleanup.len(), 1);
        assert_eq!(
            cleanup[0].root().index(),
            match shape {
                SubobjectMoveShape::Struct => 4,
                SubobjectMoveShape::FixedArray => 5,
            }
        );
        assert_eq!(cleanup[0].moved_projections().count(), 2);
    }
}

#[test]
fn aggregate_subobject_move_rejects_hostile_final_return_contexts() {
    let (sources, linear, linux, inner, outer, array) = subobject_move_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let baseline = aggregate_subobject_return_program(
        &sources,
        &linear,
        &linux,
        inner,
        outer,
        array,
        SubobjectMoveShape::Struct,
    );

    let mut public = baseline.clone();
    public.modules[0].functions[0].entry_export = Some("forged".into());

    let mut nonterminal = baseline.clone();
    let function = &mut nonterminal.modules[0].functions[0];
    let span = function.span;
    function.blocks[0].instructions.push(raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(7) },
    });

    let mut missing_topology = baseline;
    missing_topology.modules[0].functions[0].places.remove(6);
    for (label, raw) in [
        ("public final return", public),
        ("nonterminal final return", nonterminal),
        ("missing source topology", missing_topology),
    ] {
        let diagnostics =
            verify(raw, &sources, entry, linear.clone(), linux.clone()).expect_err(label);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| { matches!(diagnostic.code(), "ZRYNA-I3006" | "ZRYNA-I3010") }),
            "{label}: {diagnostics:?}"
        );
    }

    let mut parameter_root = aggregate_subobject_move_program(
        &sources,
        &linear,
        &linux,
        inner,
        outer,
        array,
        SubobjectMoveShape::Struct,
    );
    let function = &mut parameter_root.modules[0].functions[0];
    function.result = inner;
    function.blocks[0].instructions.truncate(1);
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(2), cleanup: raw::CleanupPlanId(0) };
    function.cleanup_plans[0].actions = vec![raw::DropAction::DropPlace(raw::PlaceId(0))];
    let diagnostics = verify(parameter_root, &sources, entry, linear, linux)
        .expect_err("parameter-root final return");
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"),
        "{diagnostics:?}"
    );
}

#[test]
fn aggregate_subobject_move_rejects_projected_initialize_context() {
    let (sources, linear, linux, inner, outer, array) = subobject_move_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let raw = aggregate_subobject_move_program(
        &sources,
        &linear,
        &linux,
        inner,
        outer,
        array,
        SubobjectMoveShape::Struct,
    );

    let mut projected_initialize = raw.clone();
    let function = &mut projected_initialize.modules[0].functions[0];
    let span = function.span;
    function.places[5] =
        raw::Place { id: raw::PlaceId(5), ty: outer, span, kind: raw::PlaceKind::Local(0) };
    function.places[6] = raw::Place {
        id: raw::PlaceId(6),
        ty: inner,
        span,
        kind: raw::PlaceKind::StructField { base: raw::PlaceId(5), ordinal: 0 },
    };
    function.places.push(raw::Place {
        id: raw::PlaceId(7),
        ty: raw::TypeId(2),
        span,
        kind: raw::PlaceKind::StructField { base: raw::PlaceId(6), ordinal: 0 },
    });
    function.blocks[0].instructions[1].kind =
        raw::InstructionKind::InitializePlace { place: raw::PlaceId(6), value: raw::ValueId(2) };
    function.cleanup_plans[0].actions = vec![
        raw::DropAction::DropPlace(raw::PlaceId(5)),
        raw::DropAction::DropPlace(raw::PlaceId(0)),
    ];
    let diagnostics = verify(projected_initialize, &sources, entry, linear.clone(), linux.clone())
        .expect_err("aggregate projection move initialized another projection");
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"),
        "{diagnostics:?}"
    );
}

#[test]
fn aggregate_subobject_move_rejects_direct_call_context() {
    let (sources, linear, linux, inner, outer, array) = subobject_move_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let mut direct_call = aggregate_subobject_move_program(
        &sources,
        &linear,
        &linux,
        inner,
        outer,
        array,
        SubobjectMoveShape::Struct,
    );
    let caller = &mut direct_call.modules[0].functions[0];
    let span = caller.span;
    caller.blocks[0].instructions[1] = raw::Instruction {
        result: Some(raw::ValueDefinition { id: raw::ValueId(3), ty: raw::TypeId(1), span }),
        span,
        kind: raw::InstructionKind::DirectCall {
            callee: raw::FunctionId { module: raw::ModuleId(0), declaration: 1 },
            arguments: vec![
                raw::CallArgument::Value(raw::ValueId(2)),
                raw::CallArgument::Value(raw::ValueId(1)),
            ],
            cleanup: raw::CleanupPlanId(0),
        },
    };
    caller.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(3), cleanup: raw::CleanupPlanId(1) };
    caller.cleanup_plans = vec![
        raw::CleanupPlan {
            id: raw::CleanupPlanId(0),
            span,
            actions: vec![raw::DropAction::DropPlace(raw::PlaceId(0))],
        },
        raw::CleanupPlan {
            id: raw::CleanupPlanId(1),
            span,
            actions: vec![raw::DropAction::DropPlace(raw::PlaceId(0))],
        },
    ];
    direct_call.modules[0].functions.push(raw::Function {
        id: raw::FunctionId { module: raw::ModuleId(0), declaration: 1 },
        entry_export: None,
        span,
        parameters: vec![
            raw::ValueDefinition { id: raw::ValueId(0), ty: inner, span },
            raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span },
        ],
        borrow_parameters: vec![],
        result: raw::TypeId(1),
        places: vec![
            raw::Place { id: raw::PlaceId(0), ty: inner, span, kind: raw::PlaceKind::Parameter(0) },
            raw::Place {
                id: raw::PlaceId(1),
                ty: raw::TypeId(2),
                span,
                kind: raw::PlaceKind::StructField { base: raw::PlaceId(0), ordinal: 0 },
            },
        ],
        blocks: vec![raw::Block {
            id: raw::BlockId(0),
            parameters: vec![],
            instructions: vec![],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Return {
                    value: raw::ValueId(1),
                    cleanup: raw::CleanupPlanId(0),
                },
            }],
        }],
        cleanup_plans: vec![raw::CleanupPlan {
            id: raw::CleanupPlanId(0),
            span,
            actions: vec![raw::DropAction::DropPlace(raw::PlaceId(0))],
        }],
    });
    let diagnostics = verify(direct_call, &sources, entry, linear, linux)
        .expect_err("aggregate projection move consumed by direct call");
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"),
        "{diagnostics:?}"
    );
}

#[test]
fn aggregate_subobject_move_rejects_cfg_context() {
    let (sources, linear, linux, inner, outer, array) = subobject_move_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let mut raw = aggregate_subobject_move_program(
        &sources,
        &linear,
        &linux,
        inner,
        outer,
        array,
        SubobjectMoveShape::Struct,
    );
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Jump(raw::Edge { target: raw::BlockId(1), arguments: vec![] });
    function.blocks.push(raw::Block {
        id: raw::BlockId(1),
        parameters: vec![],
        instructions: vec![],
        terminators: vec![raw::SpannedTerminator {
            span,
            kind: raw::Terminator::Return {
                value: raw::ValueId(1),
                cleanup: raw::CleanupPlanId(0),
            },
        }],
    });
    let diagnostics =
        verify(raw, &sources, entry, linear, linux).expect_err("aggregate move crossed CFG");
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"),
        "{diagnostics:?}"
    );
}

#[test]
fn aggregate_subobject_move_rejects_second_disjoint_site() {
    let (sources, linear, linux, inner, outer, array) = subobject_move_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let mut raw = aggregate_subobject_move_program(
        &sources,
        &linear,
        &linux,
        inner,
        outer,
        array,
        SubobjectMoveShape::FixedArray,
    );
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    function.places.extend([
        raw::Place {
            id: raw::PlaceId(7),
            ty: inner,
            span,
            kind: raw::PlaceKind::FixedArrayConstant { base: raw::PlaceId(0), index: 1 },
        },
        raw::Place {
            id: raw::PlaceId(8),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(7), ordinal: 0 },
        },
        raw::Place {
            id: raw::PlaceId(9),
            ty: inner,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(3)),
        },
        raw::Place {
            id: raw::PlaceId(10),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(9), ordinal: 0 },
        },
        raw::Place { id: raw::PlaceId(11), ty: inner, span, kind: raw::PlaceKind::Local(1) },
        raw::Place {
            id: raw::PlaceId(12),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(11), ordinal: 0 },
        },
    ]);
    function.blocks[0].instructions.extend([
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(3), ty: inner, span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(7) },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::InitializePlace {
                place: raw::PlaceId(11),
                value: raw::ValueId(3),
            },
        },
    ]);
    function.cleanup_plans[0].actions = vec![
        raw::DropAction::DropPlace(raw::PlaceId(11)),
        raw::DropAction::DropPlace(raw::PlaceId(5)),
        raw::DropAction::DropPlace(raw::PlaceId(0)),
    ];
    let diagnostics =
        verify(raw, &sources, entry, linear, linux).expect_err("second disjoint subobject move");
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"),
        "{diagnostics:?}"
    );
}

#[test]
fn aggregate_subobject_move_rejects_duplicate_and_overlapping_moves() {
    let (sources, linear, linux, inner, outer, array) = subobject_move_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let raw = aggregate_subobject_move_program(
        &sources,
        &linear,
        &linux,
        inner,
        outer,
        array,
        SubobjectMoveShape::Struct,
    );

    let mut duplicate = raw.clone();
    let function = &mut duplicate.modules[0].functions[0];
    let span = function.span;
    function.places.push(raw::Place {
        id: raw::PlaceId(7),
        ty: inner,
        span,
        kind: raw::PlaceKind::Temporary(raw::ValueId(3)),
    });
    function.blocks[0].instructions.insert(
        1,
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(3), ty: inner, span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(1) },
        },
    );
    let diagnostics = verify(duplicate, &sources, entry, linear.clone(), linux.clone())
        .expect_err("duplicate aggregate projection move");
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"),
        "{diagnostics:?}"
    );

    let mut child_after_parent = raw.clone();
    let function = &mut child_after_parent.modules[0].functions[0];
    let span = function.span;
    function.places.push(raw::Place {
        id: raw::PlaceId(7),
        ty: raw::TypeId(2),
        span,
        kind: raw::PlaceKind::Temporary(raw::ValueId(3)),
    });
    function.blocks[0].instructions.insert(
        1,
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(3), ty: raw::TypeId(2), span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(2) },
        },
    );
    let diagnostics = verify(child_after_parent, &sources, entry, linear.clone(), linux.clone())
        .expect_err("child move after aggregate parent move");
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"),
        "{diagnostics:?}"
    );

    let mut parent_after_child = raw;
    let function = &mut parent_after_child.modules[0].functions[0];
    let span = function.span;
    function.blocks[0].instructions[0].result =
        Some(raw::ValueDefinition { id: raw::ValueId(3), ty: inner, span });
    function.places[3].kind = raw::PlaceKind::Temporary(raw::ValueId(3));
    if let raw::InstructionKind::InitializePlace { value, .. } =
        &mut function.blocks[0].instructions[1].kind
    {
        *value = raw::ValueId(3);
    }
    function.places.push(raw::Place {
        id: raw::PlaceId(7),
        ty: raw::TypeId(2),
        span,
        kind: raw::PlaceKind::Temporary(raw::ValueId(2)),
    });
    function.blocks[0].instructions.insert(
        0,
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(2), span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(2) },
        },
    );
    let diagnostics = verify(parent_after_child, &sources, entry, linear, linux)
        .expect_err("aggregate parent move after child move");
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"),
        "{diagnostics:?}"
    );
}

#[test]
fn enum_payload_move_follows_one_exact_refined_arm_and_returns_the_local() {
    let (sources, linear, linux, payload, enum_ty) = enum_payload_move_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let raw = enum_payload_move_program(&sources, &linear, &linux, payload, enum_ty);
    let verified =
        verify(raw, &sources, entry, linear, linux).expect("one-variant aggregate payload move");
    let function = verified.modules().next().expect("module").functions().next().expect("function");
    assert_eq!(function.places().count(), 6, "one payload descendant plus five route roots");
    let arm = function.blocks().nth(1).expect("payload arm");
    assert_eq!(
        arm.instructions().map(super::VerifiedInstruction::kind).collect::<Vec<_>>(),
        [
            VerifiedInstructionKind::MoveFromPlace,
            VerifiedInstructionKind::InitializePlace,
            VerifiedInstructionKind::DropPlace,
        ]
    );
    let continuation = function.blocks().nth(2).expect("continuation");
    assert_eq!(
        continuation.instructions().map(super::VerifiedInstruction::kind).collect::<Vec<_>>(),
        [VerifiedInstructionKind::MoveFromPlace]
    );
    assert_eq!(continuation.terminator().derived_drop_actions().count(), 0);
}

#[test]
#[allow(clippy::too_many_lines)]
fn enum_payload_move_rejects_extra_parameter_place_value_and_cleanup_metadata() {
    let (sources, linear, linux, payload, enum_ty) = enum_payload_move_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let raw = enum_payload_move_program(&sources, &linear, &linux, payload, enum_ty);

    let mut extra_parameter = raw.clone();
    let function = &mut extra_parameter.modules[0].functions[0];
    let span = function.span;
    function.parameters.push(raw::ValueDefinition {
        id: raw::ValueId(1),
        ty: raw::TypeId(1),
        span,
    });
    if let Some(result) = &mut function.blocks[1].instructions[0].result {
        result.id = raw::ValueId(2);
    }
    if let raw::PlaceKind::Temporary(value) = &mut function.places[3].kind {
        *value = raw::ValueId(2);
    }
    if let raw::InstructionKind::InitializePlace { value, .. } =
        &mut function.blocks[1].instructions[1].kind
    {
        *value = raw::ValueId(2);
    }
    if let Some(result) = &mut function.blocks[2].instructions[0].result {
        result.id = raw::ValueId(3);
    }
    if let raw::PlaceKind::Temporary(value) = &mut function.places[5].kind {
        *value = raw::ValueId(3);
    }
    if let raw::Terminator::Return { value, .. } = &mut function.blocks[2].terminators[0].kind {
        *value = raw::ValueId(3);
    }
    let diagnostics = verify(extra_parameter, &sources, entry, linear.clone(), linux.clone())
        .expect_err("extra Copy parameter and value metadata");
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code() == "ZRYNA-I3010"
                && diagnostic.message().contains("enum payload move escapes")
        }),
        "{diagnostics:?}"
    );

    let mut extra_place = raw.clone();
    let span = extra_place.modules[0].functions[0].span;
    extra_place.modules[0].functions[0].places.push(raw::Place {
        id: raw::PlaceId(6),
        ty: raw::TypeId(1),
        span,
        kind: raw::PlaceKind::Local(1),
    });
    let diagnostics = verify(extra_place, &sources, entry, linear.clone(), linux.clone())
        .expect_err("extra unused Copy local place");
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"),
        "{diagnostics:?}"
    );

    let mut extra_borrow = raw.clone();
    let span = extra_borrow.modules[0].functions[0].span;
    extra_borrow.modules[0].functions[0].borrow_parameters.push(raw::BorrowParameter {
        id: raw::BorrowId(0),
        referent: raw::TypeId(1),
        access: raw::BorrowAccess::Shared,
        span,
    });
    let diagnostics = verify(extra_borrow, &sources, entry, linear.clone(), linux.clone())
        .expect_err("extra unused Copy borrow parameter metadata");
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code() == "ZRYNA-I3011"
                && diagnostic.message().contains("borrow parameter is not used")
        }),
        "{diagnostics:?}"
    );

    let mut extra_cleanup = raw;
    let span = extra_cleanup.modules[0].functions[0].span;
    extra_cleanup.modules[0].functions[0].cleanup_plans.push(raw::CleanupPlan {
        id: raw::CleanupPlanId(1),
        span,
        actions: vec![],
    });
    let diagnostics = verify(extra_cleanup, &sources, entry, linear, linux)
        .expect_err("extra unused cleanup metadata");
    assert!(
        diagnostics.iter().any(|diagnostic| matches!(
            diagnostic.code(),
            "ZRYNA-I3008" | "ZRYNA-I3010" | "ZRYNA-I3012"
        )),
        "{diagnostics:?}"
    );
}

#[test]
fn enum_payload_move_rejects_inactive_variant_and_incomplete_topology() {
    let (sources, linear, linux, payload, enum_ty) = enum_payload_move_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let raw = enum_payload_move_program(&sources, &linear, &linux, payload, enum_ty);

    let mut inactive = raw.clone();
    if let raw::PlaceKind::EnumPayload { variant, .. } =
        &mut inactive.modules[0].functions[0].places[1].kind
    {
        *variant = 1;
    }
    let diagnostics = verify(inactive, &sources, entry, linear.clone(), linux.clone())
        .expect_err("inactive payload variant");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic.code(), "ZRYNA-I3006" | "ZRYNA-I3010")),
        "{diagnostics:?}"
    );

    let mut incomplete = raw;
    incomplete.modules[0].functions[0].places[2].kind = raw::PlaceKind::Local(9);
    let diagnostics = verify(incomplete, &sources, entry, linear, linux)
        .expect_err("payload topology missing its String field");
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code() == "ZRYNA-I3010"
                && diagnostic.message().contains("enum payload move escapes")
        }),
        "{diagnostics:?}"
    );
}

#[test]
fn enum_payload_move_rejects_noncanonical_cfg_use_and_drop_order() {
    let (sources, linear, linux, payload, enum_ty) = enum_payload_move_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let raw = enum_payload_move_program(&sources, &linear, &linux, payload, enum_ty);

    let mut noncanonical_entry = raw.clone();
    let span = noncanonical_entry.modules[0].functions[0].span;
    noncanonical_entry.modules[0].functions[0].blocks[0].instructions.push(raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(0) },
    });
    let diagnostics = verify(noncanonical_entry, &sources, entry, linear.clone(), linux.clone())
        .expect_err("entry work before enum match");
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"),
        "{diagnostics:?}"
    );

    let mut non_dominating = raw.clone();
    if let raw::Terminator::EnumMatch { arms, .. } =
        &mut non_dominating.modules[0].functions[0].blocks[0].terminators[0].kind
    {
        arms[0].edge.target = raw::BlockId(2);
    }
    let diagnostics = verify(non_dominating, &sources, entry, linear.clone(), linux.clone())
        .expect_err("payload arm is not dominated by the matching edge");
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3007"),
        "{diagnostics:?}"
    );

    let mut continuation_cycle = raw.clone();
    if let raw::Terminator::Jump(edge) =
        &mut continuation_cycle.modules[0].functions[0].blocks[1].terminators[0].kind
    {
        edge.target = raw::BlockId(0);
    }
    let diagnostics = verify(continuation_cycle, &sources, entry, linear.clone(), linux.clone())
        .expect_err("payload arm jumps back to the entry");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic.code(), "ZRYNA-I3007" | "ZRYNA-I3010")),
        "{diagnostics:?}"
    );

    let mut extra_use = raw.clone();
    let duplicate = extra_use.modules[0].functions[0].blocks[1].instructions[1].clone();
    extra_use.modules[0].functions[0].blocks[1].instructions.insert(2, duplicate);
    let diagnostics = verify(extra_use, &sources, entry, linear.clone(), linux.clone())
        .expect_err("payload result has an extra use");
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"),
        "{diagnostics:?}"
    );

    let mut forged_source = raw.clone();
    if let raw::InstructionKind::MoveFromPlace { place } =
        &mut forged_source.modules[0].functions[0].blocks[1].instructions[0].kind
    {
        *place = raw::PlaceId(4);
    }
    let diagnostics = verify(forged_source, &sources, entry, linear.clone(), linux.clone())
        .expect_err("uninitialized local forged as the payload source");
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"),
        "{diagnostics:?}"
    );

    for (mut invalid, reason) in [
        (raw.clone(), "enum root drop omitted"),
        (raw, "enum root dropped before local initialization"),
    ] {
        if reason.contains("omitted") {
            invalid.modules[0].functions[0].blocks[1].instructions.pop();
        } else {
            invalid.modules[0].functions[0].blocks[1].instructions.swap(1, 2);
        }
        let diagnostics =
            verify(invalid, &sources, entry, linear.clone(), linux.clone()).expect_err(reason);
        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"),
            "{reason}: {diagnostics:?}"
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn enum_payload_move_rejects_edge_transfer_cleanup_forgery_and_second_move() {
    let (sources, linear, linux, payload, enum_ty) = enum_payload_move_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let raw = enum_payload_move_program(&sources, &linear, &linux, payload, enum_ty);

    let mut edge_transfer = raw.clone();
    let span = edge_transfer.modules[0].functions[0].span;
    edge_transfer.modules[0].functions[0].blocks[2].parameters.push(raw::ValueDefinition {
        id: raw::ValueId(2),
        ty: payload,
        span,
    });
    edge_transfer.modules[0].functions[0].places.extend([
        raw::Place {
            id: raw::PlaceId(6),
            ty: payload,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(2)),
        },
        raw::Place {
            id: raw::PlaceId(7),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(6), ordinal: 0 },
        },
    ]);
    if let raw::PlaceKind::Temporary(value) =
        &mut edge_transfer.modules[0].functions[0].places[5].kind
    {
        *value = raw::ValueId(3);
    }
    if let Some(result) =
        &mut edge_transfer.modules[0].functions[0].blocks[2].instructions[0].result
    {
        result.id = raw::ValueId(3);
    }
    if let raw::Terminator::Return { value, .. } =
        &mut edge_transfer.modules[0].functions[0].blocks[2].terminators[0].kind
    {
        *value = raw::ValueId(3);
    }
    if let raw::Terminator::Jump(edge) =
        &mut edge_transfer.modules[0].functions[0].blocks[1].terminators[0].kind
    {
        edge.arguments.push(raw::ValueId(1));
    }
    let diagnostics = verify(edge_transfer, &sources, entry, linear.clone(), linux.clone())
        .expect_err("payload owner carried across the edge");
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"),
        "{diagnostics:?}"
    );

    let mut forged_cleanup = raw.clone();
    forged_cleanup.modules[0].functions[0].cleanup_plans[0]
        .actions
        .push(raw::DropAction::DropPlace(raw::PlaceId(4)));
    let diagnostics = verify(forged_cleanup, &sources, entry, linear.clone(), linux.clone())
        .expect_err("continuation return cleanup is not empty");
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3010"),
        "{diagnostics:?}"
    );

    let mut second_move = raw;
    let function = &mut second_move.modules[0].functions[0];
    function.places.extend([
        raw::Place {
            id: raw::PlaceId(6),
            ty: payload,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(2)),
        },
        raw::Place {
            id: raw::PlaceId(7),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(6), ordinal: 0 },
        },
    ]);
    function.blocks[1].instructions.insert(
        1,
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(2), ty: payload, span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(1) },
        },
    );
    for id in 3..35 {
        function.blocks[1].instructions.push(raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(id), ty: raw::TypeId(1), span }),
            span,
            kind: raw::InstructionKind::I32Literal(i32::try_from(id).expect("small noise id")),
        });
    }
    if let raw::PlaceKind::Temporary(value) = &mut function.places[5].kind {
        *value = raw::ValueId(35);
    }
    if let Some(result) = &mut function.blocks[2].instructions[0].result {
        result.id = raw::ValueId(35);
    }
    if let raw::Terminator::Return { value, .. } = &mut function.blocks[2].terminators[0].kind {
        *value = raw::ValueId(35);
    }
    let diagnostics =
        verify(second_move, &sources, entry, linear, linux).expect_err("second enum payload move");
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code() == "ZRYNA-I3010"
                && diagnostic.message().contains("more than one enum payload move")
        }),
        "{diagnostics:?}"
    );
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
        kind: raw::PlaceKind::Local(0),
    }];
    caller.blocks[0].instructions = vec![
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
                cleanup: raw::CleanupPlanId(0),
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
    if let raw::Terminator::Return { cleanup, .. } = &mut caller.blocks[0].terminators[0].kind {
        *cleanup = raw::CleanupPlanId(1);
    }
    caller.cleanup_plans.push(raw::CleanupPlan {
        id: raw::CleanupPlanId(1),
        span,
        actions: vec![],
    });
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
            instructions: vec![raw::Instruction {
                result: Some(raw::ValueDefinition {
                    id: raw::ValueId(1),
                    ty: raw::TypeId(1),
                    span,
                }),
                span,
                kind: raw::InstructionKind::BorrowRead { borrow: raw::BorrowId(0) },
            }],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Return {
                    value: raw::ValueId(1),
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
#[allow(clippy::too_many_lines)]
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
        kind: raw::PlaceKind::Local(0),
    }];
    caller.blocks[0].instructions = vec![
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
                cleanup: raw::CleanupPlanId(0),
            },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::EndBorrow { borrow: raw::BorrowId(0) },
        },
    ];
    caller.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(1), cleanup: raw::CleanupPlanId(1) };
    caller.cleanup_plans.push(raw::CleanupPlan {
        id: raw::CleanupPlanId(1),
        span,
        actions: vec![],
    });
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
            instructions: vec![
                raw::Instruction {
                    result: Some(raw::ValueDefinition {
                        id: raw::ValueId(1),
                        ty: raw::TypeId(1),
                        span,
                    }),
                    span,
                    kind: raw::InstructionKind::BorrowRead { borrow: raw::BorrowId(0) },
                },
                raw::Instruction {
                    result: Some(raw::ValueDefinition {
                        id: raw::ValueId(2),
                        ty: raw::TypeId(1),
                        span,
                    }),
                    span,
                    kind: raw::InstructionKind::BorrowRead { borrow: raw::BorrowId(1) },
                },
            ],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Return {
                    value: raw::ValueId(2),
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
            cleanup: Some(raw::CleanupPlanId(0)),
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
#[ignore = "proportional exact/+1 8 MiB String-literal boundary retained by full preflight"]
fn string_literal_program_byte_budget_accepts_exact_and_rejects_first_extra() {
    let (sources, linear, linux) = authorities();
    let mut exact =
        string_literal_program(&sources, &linear, &linux, vec![b'a'; MAX_STRING_LITERAL_BYTES / 2]);
    let span = exact.modules[0].functions[0].span;
    exact.modules[0].functions[0].blocks[0].instructions.push(raw::Instruction {
        result: Some(raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(2), span }),
        span,
        kind: raw::InstructionKind::StringFromUtf8 {
            bytes: vec![b'b'; MAX_STRING_LITERAL_BYTES / 2],
            cleanup: raw::CleanupPlanId(0),
        },
    });
    assert!(preflight_codes(&exact, &linear).is_empty());

    let raw::InstructionKind::StringFromUtf8 { bytes, .. } =
        &mut exact.modules[0].functions[0].blocks[0].instructions[1].kind
    else {
        panic!("second String literal");
    };
    bytes.push(b'b');
    assert_eq!(preflight_codes(&exact, &linear), ["ZRYNA-I3201"]);
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

    let mut mixed = program(&sources, &linear, &linux);
    let span = mixed.modules[0].functions[0].span;
    mixed.modules[0].functions[0].cleanup_plans[0].actions =
        vec![raw::DropAction::DropPlace(raw::PlaceId(0)); MAX_DROP_ACTIONS_PER_FUNCTION - 1];
    mixed.modules[0].functions[0].blocks[0].instructions.push(raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(0) },
    });
    assert!(preflight_codes(&mixed, &linear).is_empty());
    mixed.modules[0].functions[0].blocks[0].instructions.push(raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(0) },
    });
    assert_eq!(preflight_codes(&mixed, &linear), ["ZRYNA-I3201"]);
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
    let (sources, linear, linux) = pair_authorities();
    let mut raw = program(&sources, &linear, &linux);
    raw.modules[0].data_declarations = 1;
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
            ty: raw::TypeId(3),
            span,
            kind: raw::PlaceKind::Local(0),
        },
        raw::Place {
            id: raw::PlaceId(2),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(1), ordinal: 0 },
        },
        raw::Place {
            id: raw::PlaceId(3),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(1), ordinal: 1 },
        },
    ];
    function.blocks[0].instructions = vec![raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::InitializePlace {
            place: raw::PlaceId(2),
            value: raw::ValueId(0),
        },
    }];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(1), cleanup: raw::CleanupPlanId(0) };
    function.cleanup_plans[0].actions = vec![raw::DropAction::DropPlace(raw::PlaceId(1))];
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
        [2]
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn ordered_projection_initialization_promotes_root_and_rejects_holes() {
    let (sources, linear, linux) = pair_authorities();
    let mut raw = program(&sources, &linear, &linux);
    raw.modules[0].data_declarations = 1;
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(2), span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(2), span },
        raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(1), span },
    ];
    function.result = raw::TypeId(1);
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
            kind: raw::PlaceKind::Parameter(1),
        },
        raw::Place {
            id: raw::PlaceId(2),
            ty: raw::TypeId(3),
            span,
            kind: raw::PlaceKind::Local(0),
        },
        raw::Place {
            id: raw::PlaceId(3),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(2), ordinal: 0 },
        },
        raw::Place {
            id: raw::PlaceId(4),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(2), ordinal: 1 },
        },
    ];
    function.blocks[0].instructions = vec![
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::InitializePlace {
                place: raw::PlaceId(3),
                value: raw::ValueId(0),
            },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::InitializePlace {
                place: raw::PlaceId(4),
                value: raw::ValueId(1),
            },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::BeginBorrow(raw::BorrowDefinition {
                id: raw::BorrowId(0),
                place: raw::PlaceId(2),
                access: raw::BorrowAccess::Shared,
                span,
            }),
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::EndBorrow { borrow: raw::BorrowId(0) },
        },
    ];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(2), cleanup: raw::CleanupPlanId(0) };
    function.cleanup_plans[0].actions = vec![raw::DropAction::DropPlace(raw::PlaceId(2))];
    let entry = sources.verify_file_id(0).expect("entry");
    let verified = verify(raw.clone(), &sources, entry, linear.clone(), linux.clone())
        .expect("ordered fields promote the root");
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
        .expect("drop");
    assert_eq!(
        action.initialized_projections().map(super::PlaceIdentity::index).collect::<Vec<_>>(),
        [3, 4]
    );

    let mut reversed = raw.clone();
    reversed.modules[0].functions[0].blocks[0].instructions.swap(0, 1);
    assert!(
        verify(reversed, &sources, entry, linear.clone(), linux.clone())
            .expect_err("out-of-order fields")
            .iter()
            .any(|diagnostic| diagnostic.code() == "ZRYNA-I3013")
    );

    let mut duplicate = raw.clone();
    let repeated = duplicate.modules[0].functions[0].blocks[0].instructions[0].clone();
    duplicate.modules[0].functions[0].blocks[0].instructions.insert(1, repeated);
    assert!(
        verify(duplicate, &sources, entry, linear.clone(), linux.clone())
            .expect_err("duplicate field commit")
            .iter()
            .any(|diagnostic| diagnostic.code() == "ZRYNA-I3010")
    );

    let mut incomplete = raw;
    let function = &mut incomplete.modules[0].functions[0];
    function.places.remove(4);
    function.blocks[0].instructions.remove(1);
    function.blocks[0].instructions[1] = raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::BeginBorrow(raw::BorrowDefinition {
            id: raw::BorrowId(0),
            place: raw::PlaceId(2),
            access: raw::BorrowAccess::Shared,
            span,
        }),
    };
    assert!(
        verify(incomplete, &sources, entry, linear, linux)
            .expect_err("incomplete projection inventory")
            .iter()
            .any(|diagnostic| diagnostic.code() == "ZRYNA-I3013")
    );
}

#[test]
fn initialize_enum_payload_establishes_exact_active_variant() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(2), span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span },
    ];
    function.result = raw::TypeId(1);
    function.places = vec![
        raw::Place {
            id: raw::PlaceId(0),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Parameter(0),
        },
        raw::Place {
            id: raw::PlaceId(1),
            ty: raw::TypeId(4),
            span,
            kind: raw::PlaceKind::Local(0),
        },
        raw::Place {
            id: raw::PlaceId(2),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::EnumPayload { base: raw::PlaceId(1), variant: 0 },
        },
        raw::Place {
            id: raw::PlaceId(3),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(2)),
        },
    ];
    function.blocks[0].instructions = vec![
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::InitializePlace {
                place: raw::PlaceId(2),
                value: raw::ValueId(0),
            },
        },
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(2), span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(2) },
        },
    ];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(1), cleanup: raw::CleanupPlanId(0) };
    function.cleanup_plans[0].actions = vec![
        raw::DropAction::DropPlace(raw::PlaceId(3)),
        raw::DropAction::DropPlace(raw::PlaceId(1)),
    ];
    let entry = sources.verify_file_id(0).expect("entry");
    let verified = verify(raw, &sources, entry, linear, linux).expect("payload activation");
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
        .nth(1)
        .expect("enum drop");
    assert_eq!(action.active_variant(), Some(0));
    assert_eq!(
        action
            .active_variants()
            .map(|active| (active.place().index(), active.variant()))
            .collect::<Vec<_>>(),
        [(1, 0)]
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn copy_prefix_creates_owned_root_obligation_and_fixed_array_is_prefix_only() {
    let (sources, linear, linux, struct_ty, array_ty, _) = mixed_aggregate_authorities();
    let mut raw = program(&sources, &linear, &linux);
    raw.modules[0].data_declarations = 2;
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(1), span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(2), span },
        raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(1), span },
    ];
    function.result = raw::TypeId(1);
    function.places = vec![
        raw::Place {
            id: raw::PlaceId(0),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Parameter(1),
        },
        raw::Place { id: raw::PlaceId(1), ty: struct_ty, span, kind: raw::PlaceKind::Local(0) },
        raw::Place {
            id: raw::PlaceId(2),
            ty: raw::TypeId(1),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(1), ordinal: 0 },
        },
        raw::Place {
            id: raw::PlaceId(3),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(1), ordinal: 1 },
        },
    ];
    function.blocks[0].instructions = vec![raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::InitializePlace {
            place: raw::PlaceId(2),
            value: raw::ValueId(0),
        },
    }];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(2), cleanup: raw::CleanupPlanId(0) };
    function.cleanup_plans[0].actions = vec![
        raw::DropAction::DropPlace(raw::PlaceId(1)),
        raw::DropAction::DropPlace(raw::PlaceId(0)),
    ];
    let entry = sources.verify_file_id(0).expect("entry");
    verify(raw, &sources, entry, linear.clone(), linux.clone())
        .expect("Copy prefix still owns its non-Copy root");

    let mut array = program(&sources, &linear, &linux);
    array.modules[0].data_declarations = 2;
    let function = &mut array.modules[0].functions[0];
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(2), span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(2), span },
        raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(1), span },
    ];
    function.result = raw::TypeId(1);
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
            kind: raw::PlaceKind::Parameter(1),
        },
        raw::Place { id: raw::PlaceId(2), ty: array_ty, span, kind: raw::PlaceKind::Local(0) },
        raw::Place {
            id: raw::PlaceId(3),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::FixedArrayConstant { base: raw::PlaceId(2), index: 0 },
        },
        raw::Place {
            id: raw::PlaceId(4),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::FixedArrayConstant { base: raw::PlaceId(2), index: 1 },
        },
    ];
    function.blocks[0].instructions = vec![
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::InitializePlace {
                place: raw::PlaceId(3),
                value: raw::ValueId(0),
            },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::InitializePlace {
                place: raw::PlaceId(4),
                value: raw::ValueId(1),
            },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::BeginBorrow(raw::BorrowDefinition {
                id: raw::BorrowId(0),
                place: raw::PlaceId(2),
                access: raw::BorrowAccess::Shared,
                span,
            }),
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::EndBorrow { borrow: raw::BorrowId(0) },
        },
    ];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(2), cleanup: raw::CleanupPlanId(0) };
    function.cleanup_plans[0].actions = vec![raw::DropAction::DropPlace(raw::PlaceId(2))];
    verify(array.clone(), &sources, entry, linear.clone(), linux.clone())
        .expect("ordered array promotes root");
    array.modules[0].functions[0].blocks[0].instructions.swap(0, 1);
    assert!(
        verify(array, &sources, entry, linear, linux)
            .expect_err("array hole")
            .iter()
            .any(|diagnostic| diagnostic.code() == "ZRYNA-I3013")
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn nested_enum_payload_prefix_activates_one_variant_and_rejects_another() {
    let (sources, linear, linux, struct_ty, _, enum_ty) = mixed_aggregate_authorities();
    let mut raw = program(&sources, &linear, &linux);
    raw.modules[0].data_declarations = 2;
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(1), span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(2), span },
        raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(1), span },
    ];
    function.result = raw::TypeId(1);
    function.places = vec![
        raw::Place {
            id: raw::PlaceId(0),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Parameter(1),
        },
        raw::Place { id: raw::PlaceId(1), ty: enum_ty, span, kind: raw::PlaceKind::Local(0) },
        raw::Place {
            id: raw::PlaceId(2),
            ty: struct_ty,
            span,
            kind: raw::PlaceKind::EnumPayload { base: raw::PlaceId(1), variant: 0 },
        },
        raw::Place {
            id: raw::PlaceId(3),
            ty: raw::TypeId(1),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(2), ordinal: 0 },
        },
        raw::Place {
            id: raw::PlaceId(4),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(2), ordinal: 1 },
        },
        raw::Place {
            id: raw::PlaceId(5),
            ty: struct_ty,
            span,
            kind: raw::PlaceKind::EnumPayload { base: raw::PlaceId(1), variant: 1 },
        },
        raw::Place {
            id: raw::PlaceId(6),
            ty: raw::TypeId(1),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(5), ordinal: 0 },
        },
        raw::Place {
            id: raw::PlaceId(7),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(5), ordinal: 1 },
        },
    ];
    function.blocks[0].instructions = vec![raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::InitializePlace {
            place: raw::PlaceId(3),
            value: raw::ValueId(0),
        },
    }];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(2), cleanup: raw::CleanupPlanId(0) };
    function.cleanup_plans[0].actions = vec![
        raw::DropAction::DropPlace(raw::PlaceId(1)),
        raw::DropAction::DropPlace(raw::PlaceId(0)),
    ];
    let entry = sources.verify_file_id(0).expect("entry");
    let verified = verify(raw.clone(), &sources, entry, linear.clone(), linux.clone())
        .expect("nested active payload prefix");
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
        .expect("enum root");
    assert_eq!(action.active_variant(), Some(0));
    assert_eq!(
        action
            .active_variants()
            .map(|active| (active.place().index(), active.variant()))
            .collect::<Vec<_>>(),
        [(1, 0)]
    );

    let mut conflicting = raw;
    conflicting.modules[0].functions[0].blocks[0].instructions.push(raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::InitializePlace {
            place: raw::PlaceId(6),
            value: raw::ValueId(0),
        },
    });
    assert!(
        verify(conflicting, &sources, entry, linear, linux)
            .expect_err("conflicting active variant")
            .iter()
            .any(|diagnostic| diagnostic.code() == "ZRYNA-I3013")
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn whole_owner_rename_preserves_partial_projection_metadata() {
    let (sources, linear, linux) = pair_authorities();
    let mut raw = program(&sources, &linear, &linux);
    raw.modules[0].data_declarations = 1;
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
            ty: raw::TypeId(3),
            span,
            kind: raw::PlaceKind::Local(0),
        },
        raw::Place {
            id: raw::PlaceId(2),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(1), ordinal: 0 },
        },
        raw::Place {
            id: raw::PlaceId(3),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(1), ordinal: 1 },
        },
        raw::Place {
            id: raw::PlaceId(4),
            ty: raw::TypeId(3),
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(2)),
        },
        raw::Place {
            id: raw::PlaceId(5),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(4), ordinal: 0 },
        },
        raw::Place {
            id: raw::PlaceId(6),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(4), ordinal: 1 },
        },
    ];
    function.blocks[0].instructions = vec![
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::InitializePlace {
                place: raw::PlaceId(2),
                value: raw::ValueId(0),
            },
        },
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(3), span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(1) },
        },
    ];
    function.blocks[0].terminators[0].kind = raw::Terminator::Trap {
        identity: raw::TrapIdentity::AllocationV1,
        cleanup: raw::CleanupPlanId(0),
    };
    function.cleanup_plans[0].actions = vec![raw::DropAction::DropPlace(raw::PlaceId(4))];
    let entry = sources.verify_file_id(0).expect("entry");
    let verified = verify(raw.clone(), &sources, entry, linear.clone(), linux.clone())
        .expect("partial owner rename");
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
        [5]
    );

    let mut missing_destination = raw;
    missing_destination.modules[0].functions[0].places.remove(5);
    missing_destination.modules[0].functions[0].places[5].id = raw::PlaceId(5);
    let diagnostics = verify(missing_destination, &sources, entry, linear, linux)
        .expect_err("partial rename without matching destination projection metadata");
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code() == "ZRYNA-I3010"
            && diagnostic
                .message()
                .contains("partial owner rename requires exact matching projection metadata")
    }));
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
        kind: raw::PlaceKind::Local(0),
    }];
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
            cleanup: raw::CleanupPlanId(0),
        },
    };
    let mut calls = seed;
    calls.modules[0].functions[0].blocks[0].instructions = vec![call.clone(); MAX_CALL_EDGES];
    assert!(preflight_codes(&calls, &linear).is_empty());
    calls.modules[0].functions[0].blocks[0].instructions.push(call);
    assert_eq!(preflight_codes(&calls, &linear), ["ZRYNA-I3201"]);
    let _ = linux;
}

#[test]
#[allow(clippy::too_many_lines)]
fn projection_replace_preserves_a_moved_sibling_in_derived_cleanup() {
    let (sources, linear, linux) = pair_authorities();
    let mut raw = program(&sources, &linear, &linux);
    raw.modules[0].data_declarations = 1;
    let span = raw.modules[0].functions[0].span;
    let function = &mut raw.modules[0].functions[0];
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(3), span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(2), span },
        raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(1), span },
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
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(0), ordinal: 1 },
        },
        raw::Place {
            id: raw::PlaceId(3),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Parameter(1),
        },
        raw::Place {
            id: raw::PlaceId(4),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(3)),
        },
    ];
    function.blocks[0].instructions = vec![
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(3), ty: raw::TypeId(2), span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(1) },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::ReplacePlace {
                place: raw::PlaceId(2),
                value: raw::ValueId(1),
            },
        },
    ];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(2), cleanup: raw::CleanupPlanId(0) };
    function.cleanup_plans = vec![raw::CleanupPlan {
        id: raw::CleanupPlanId(0),
        span,
        actions: vec![
            raw::DropAction::DropPlace(raw::PlaceId(4)),
            raw::DropAction::DropPlace(raw::PlaceId(0)),
        ],
    }];

    let entry = sources.verify_file_id(0).expect("entry");
    let verified = verify(raw, &sources, entry, linear, linux).expect("projection replacement");
    let replacement = verified
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
        .nth(1)
        .expect("replacement");
    assert_eq!(replacement.kind(), VerifiedInstructionKind::ReplacePlace);
    let replaced_field =
        replacement.derived_drop_actions().next().expect("old projected destination action");
    assert_eq!(replaced_field.root().index(), 2);
    assert_eq!(replaced_field.kind(), VerifiedDropActionKind::Place);
    assert_eq!(replaced_field.initialized_projections().count(), 0);
    assert_eq!(replaced_field.moved_projections().count(), 0);
    assert_eq!(replaced_field.active_variant(), None);
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
        .nth(1)
        .expect("pair drop");
    assert_eq!(
        action.moved_projections().map(super::PlaceIdentity::index).collect::<Vec<_>>(),
        [1]
    );
    assert_eq!(
        action.initialized_projections().map(super::PlaceIdentity::index).collect::<Vec<_>>(),
        [2]
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn projected_enum_replace_transplants_the_replacement_variant() {
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "main.zry".into(),
        text: "export function id(value: i32): i32 { return value; }".into(),
    }])
    .expect("source map");
    let file = sources.verify_file_id(0).expect("source file");
    let span = sources.span(file, 0, 6).expect("span");
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
                span: Some(span),
                kind: raw_layout::TypeKind::Enum {
                    module: raw_layout::ModuleId(0),
                    declaration: 0,
                    variants: vec![
                        raw_layout::Variant { ordinal: 0, payload: None },
                        raw_layout::Variant { ordinal: 1, payload: Some(raw_layout::NodeId(2)) },
                    ],
                },
            },
            raw_layout::TypeNode {
                id: raw_layout::NodeId(4),
                span: Some(span),
                kind: raw_layout::TypeKind::Struct {
                    module: raw_layout::ModuleId(0),
                    declaration: 1,
                    fields: vec![raw_layout::Field { ordinal: 0, ty: raw_layout::NodeId(3) }],
                },
            },
        ],
        program_roots: vec![raw_layout::NodeId(4)],
    };
    let linear =
        zryna_layout::verify(&graph, &sources, StorageTarget::Linear32V1).expect("linear layouts");
    let linux = zryna_layout::verify(&graph, &sources, StorageTarget::LinuxX8664V1)
        .expect("native layouts");
    let enum_ty = linear
        .types()
        .find(|ty| ty.nominal_identity() == Some((0, 0)))
        .map(|ty| raw::TypeId(ty.id().index()))
        .expect("enum type");
    let wrapper_ty = linear
        .types()
        .find(|ty| ty.nominal_identity() == Some((0, 1)))
        .map(|ty| raw::TypeId(ty.id().index()))
        .expect("wrapper type");
    let mut raw = program(&sources, &linear, &linux);
    raw.modules[0].data_declarations = 2;
    let function = &mut raw.modules[0].functions[0];
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(2), span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span },
    ];
    function.result = raw::TypeId(1);
    function.places = vec![
        raw::Place { id: raw::PlaceId(0), ty: wrapper_ty, span, kind: raw::PlaceKind::Local(0) },
        raw::Place {
            id: raw::PlaceId(1),
            ty: enum_ty,
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(0), ordinal: 0 },
        },
        raw::Place {
            id: raw::PlaceId(2),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::EnumPayload { base: raw::PlaceId(1), variant: 1 },
        },
        raw::Place {
            id: raw::PlaceId(3),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Parameter(0),
        },
        raw::Place {
            id: raw::PlaceId(4),
            ty: enum_ty,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(2)),
        },
        raw::Place {
            id: raw::PlaceId(5),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::EnumPayload { base: raw::PlaceId(4), variant: 1 },
        },
        raw::Place {
            id: raw::PlaceId(6),
            ty: enum_ty,
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(3)),
        },
        raw::Place {
            id: raw::PlaceId(7),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::EnumPayload { base: raw::PlaceId(6), variant: 1 },
        },
    ];
    function.blocks[0].instructions = vec![
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(2), ty: enum_ty, span }),
            span,
            kind: raw::InstructionKind::EnumConstruct { variant: 0, payload: None, cleanup: None },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::InitializePlace {
                place: raw::PlaceId(1),
                value: raw::ValueId(2),
            },
        },
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(3), ty: enum_ty, span }),
            span,
            kind: raw::InstructionKind::EnumConstruct {
                variant: 1,
                payload: Some(raw::ValueId(0)),
                cleanup: None,
            },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::ReplacePlace {
                place: raw::PlaceId(1),
                value: raw::ValueId(3),
            },
        },
    ];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(1), cleanup: raw::CleanupPlanId(0) };
    function.cleanup_plans[0].actions = vec![raw::DropAction::DropPlace(raw::PlaceId(0))];

    let entry = sources.verify_file_id(0).expect("entry");
    let verified = verify(raw.clone(), &sources, entry, linear.clone(), linux.clone())
        .expect("projected enum replacement");
    let function = verified.modules().next().expect("module").functions().next().expect("function");
    let block = function.blocks().next().expect("block");
    let replacement = block.instructions().nth(3).expect("replacement");
    let old_value = replacement.derived_drop_actions().next().expect("old projected enum action");
    assert_eq!(old_value.root().index(), 1);
    assert_eq!(old_value.active_variant(), Some(0));
    let final_owner = block.terminator().derived_drop_actions().next().expect("wrapper drop");
    assert_eq!(
        final_owner
            .active_variants()
            .map(|active| (active.place().index(), active.variant()))
            .collect::<Vec<_>>(),
        [(1, 1)]
    );
    assert_eq!(
        final_owner.initialized_projections().map(super::PlaceIdentity::index).collect::<Vec<_>>(),
        [1, 2]
    );

    let mut partial_source = raw;
    let function = &mut partial_source.modules[0].functions[0];
    function.places.push(raw::Place {
        id: raw::PlaceId(8),
        ty: raw::TypeId(2),
        span,
        kind: raw::PlaceKind::Temporary(raw::ValueId(4)),
    });
    function.blocks[0].instructions.insert(
        3,
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(4), ty: raw::TypeId(2), span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(7) },
        },
    );
    assert!(
        verify(partial_source, &sources, entry, linear, linux)
            .expect_err("partial projected replacement source")
            .iter()
            .any(|diagnostic| diagnostic.code() == "ZRYNA-I3010")
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn root_replace_exposes_old_recursive_drop_shape_and_rejects_invalid_states() {
    let (sources, linear, linux) = pair_authorities();
    let mut raw = program(&sources, &linear, &linux);
    raw.modules[0].data_declarations = 1;
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(3), span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(3), span },
        raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(1), span },
    ];
    function.result = raw::TypeId(1);
    function.places = vec![
        raw::Place {
            id: raw::PlaceId(0),
            ty: raw::TypeId(3),
            span,
            kind: raw::PlaceKind::Parameter(0),
        },
        raw::Place {
            id: raw::PlaceId(1),
            ty: raw::TypeId(3),
            span,
            kind: raw::PlaceKind::Parameter(1),
        },
        raw::Place {
            id: raw::PlaceId(2),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(0), ordinal: 0 },
        },
        raw::Place {
            id: raw::PlaceId(3),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(0), ordinal: 1 },
        },
        raw::Place {
            id: raw::PlaceId(4),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(1), ordinal: 0 },
        },
        raw::Place {
            id: raw::PlaceId(5),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(1), ordinal: 1 },
        },
    ];
    function.blocks[0].instructions = vec![raw::Instruction {
        result: None,
        span,
        kind: raw::InstructionKind::ReplacePlace { place: raw::PlaceId(0), value: raw::ValueId(1) },
    }];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(2), cleanup: raw::CleanupPlanId(0) };
    function.cleanup_plans[0].actions = vec![raw::DropAction::DropPlace(raw::PlaceId(0))];

    let entry = sources.verify_file_id(0).expect("entry");
    let verified = verify(raw.clone(), &sources, entry, linear.clone(), linux.clone())
        .expect("root replacement");
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
        .instructions()
        .next()
        .expect("replace")
        .derived_drop_actions()
        .next()
        .expect("old destination action");
    assert_eq!(action.root().index(), 0);
    assert_eq!(
        action.initialized_projections().map(super::PlaceIdentity::index).collect::<Vec<_>>(),
        [2, 3]
    );
    assert_eq!(action.moved_projections().count(), 0);
    assert_eq!(action.active_variant(), None);

    let mut wrong_type = raw.clone();
    let raw::InstructionKind::ReplacePlace { value, .. } =
        &mut wrong_type.modules[0].functions[0].blocks[0].instructions[0].kind
    else {
        panic!("replace")
    };
    *value = raw::ValueId(2);
    assert!(
        verify(wrong_type, &sources, entry, linear.clone(), linux.clone())
            .expect_err("replacement type mismatch")
            .iter()
            .any(|diagnostic| diagnostic.code() == "ZRYNA-I3005")
    );

    let mut moved = raw;
    let function = &mut moved.modules[0].functions[0];
    function.places.push(raw::Place {
        id: raw::PlaceId(6),
        ty: raw::TypeId(3),
        span,
        kind: raw::PlaceKind::Temporary(raw::ValueId(3)),
    });
    function.blocks[0].instructions.insert(
        0,
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(3), ty: raw::TypeId(3), span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(0) },
        },
    );
    assert!(
        verify(moved, &sources, entry, linear, linux)
            .expect_err("moved replacement target")
            .iter()
            .any(|diagnostic| diagnostic.code() == "ZRYNA-I3010")
    );
}

#[derive(Clone, Copy)]
enum PartialReplaceShape {
    Struct,
    FixedArray,
}

#[derive(Clone, Copy)]
struct PartialReplaceTopology {
    destination_second: bool,
    source_second: bool,
    temporary_second: bool,
}

const EXACT_PARTIAL_REPLACE_TOPOLOGY: PartialReplaceTopology = PartialReplaceTopology {
    destination_second: true,
    source_second: true,
    temporary_second: true,
};

fn partial_replace_projection(
    shape: PartialReplaceShape,
    base: raw::PlaceId,
    index: u32,
) -> raw::PlaceKind {
    match shape {
        PartialReplaceShape::Struct => raw::PlaceKind::StructField { base, ordinal: index },
        PartialReplaceShape::FixedArray => raw::PlaceKind::FixedArrayConstant { base, index },
    }
}

fn push_partial_replace_place(
    places: &mut Vec<raw::Place>,
    ty: raw::TypeId,
    span: zryna_source::Span,
    kind: raw::PlaceKind,
) -> raw::PlaceId {
    let id = raw::PlaceId(u32::try_from(places.len()).expect("bounded partial replace place"));
    places.push(raw::Place { id, ty, span, kind });
    id
}

#[allow(clippy::too_many_arguments)]
fn partial_replace_program(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
    data_declarations: u32,
    root_ty: raw::TypeId,
    shape: PartialReplaceShape,
    topology: PartialReplaceTopology,
) -> raw::Program {
    let mut raw = program(sources, linear, linux);
    raw.modules[0].data_declarations = data_declarations;
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: root_ty, span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: root_ty, span },
        raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(1), span },
    ];
    function.result = raw::TypeId(1);

    let mut places = Vec::new();
    let destination =
        push_partial_replace_place(&mut places, root_ty, span, raw::PlaceKind::Parameter(0));
    let destination_first = push_partial_replace_place(
        &mut places,
        raw::TypeId(2),
        span,
        partial_replace_projection(shape, destination, 0),
    );
    if topology.destination_second {
        push_partial_replace_place(
            &mut places,
            raw::TypeId(2),
            span,
            partial_replace_projection(shape, destination, 1),
        );
    }
    let source =
        push_partial_replace_place(&mut places, root_ty, span, raw::PlaceKind::Parameter(1));
    let source_first = push_partial_replace_place(
        &mut places,
        raw::TypeId(2),
        span,
        partial_replace_projection(shape, source, 0),
    );
    if topology.source_second {
        push_partial_replace_place(
            &mut places,
            raw::TypeId(2),
            span,
            partial_replace_projection(shape, source, 1),
        );
    }
    let moved_leaf = push_partial_replace_place(
        &mut places,
        raw::TypeId(2),
        span,
        raw::PlaceKind::Temporary(raw::ValueId(3)),
    );
    let prepared = push_partial_replace_place(
        &mut places,
        root_ty,
        span,
        raw::PlaceKind::Temporary(raw::ValueId(4)),
    );
    push_partial_replace_place(
        &mut places,
        raw::TypeId(2),
        span,
        partial_replace_projection(shape, prepared, 0),
    );
    if topology.temporary_second {
        push_partial_replace_place(
            &mut places,
            raw::TypeId(2),
            span,
            partial_replace_projection(shape, prepared, 1),
        );
    }
    function.places = places;
    function.blocks[0].instructions = vec![
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(3), ty: raw::TypeId(2), span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: source_first },
        },
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(4), ty: root_ty, span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: source },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::ReplacePlace { place: destination, value: raw::ValueId(4) },
        },
    ];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(2), cleanup: raw::CleanupPlanId(0) };
    function.cleanup_plans[0].actions =
        vec![raw::DropAction::DropPlace(moved_leaf), raw::DropAction::DropPlace(destination)];
    let _ = destination_first;
    raw
}

fn assert_partial_replace_verified(
    raw: raw::Program,
    sources: &SourceMap,
    linear: zryna_layout::VerifiedLayouts,
    linux: zryna_layout::VerifiedLayouts,
) {
    let entry = sources.verify_file_id(0).expect("entry");
    let verified = verify(raw, sources, entry, linear, linux).expect("exact partial replacement");
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
    let old_destination = block
        .instructions()
        .nth(2)
        .expect("replacement")
        .derived_drop_actions()
        .next()
        .expect("old destination action");
    assert_eq!(old_destination.root().index(), 0);
    assert_eq!(
        old_destination
            .initialized_projections()
            .map(super::PlaceIdentity::index)
            .collect::<Vec<_>>(),
        [1, 2]
    );
    assert_eq!(old_destination.moved_projections().count(), 0);

    let cleanup = block.terminator().derived_drop_actions().collect::<Vec<_>>();
    assert_eq!(cleanup.len(), 2);
    assert_eq!(cleanup[1].root().index(), 0);
    assert_eq!(
        cleanup[1].moved_projections().map(super::PlaceIdentity::index).collect::<Vec<_>>(),
        [1]
    );
    assert_eq!(
        cleanup[1].initialized_projections().map(super::PlaceIdentity::index).collect::<Vec<_>>(),
        [2]
    );
}

#[test]
fn root_replace_accepts_exact_partial_struct_and_fixed_array_topology() {
    let (sources, linear, linux) = pair_authorities();
    let raw = partial_replace_program(
        &sources,
        &linear,
        &linux,
        1,
        raw::TypeId(3),
        PartialReplaceShape::Struct,
        EXACT_PARTIAL_REPLACE_TOPOLOGY,
    );
    assert_partial_replace_verified(raw, &sources, linear, linux);

    let (sources, linear, linux, _, array_ty, _) = mixed_aggregate_authorities();
    let raw = partial_replace_program(
        &sources,
        &linear,
        &linux,
        2,
        array_ty,
        PartialReplaceShape::FixedArray,
        EXACT_PARTIAL_REPLACE_TOPOLOGY,
    );
    assert_partial_replace_verified(raw, &sources, linear, linux);
}

#[test]
#[allow(clippy::too_many_lines)]
fn root_replace_rejects_incomplete_or_forged_partial_topology() {
    let (sources, linear, linux) = pair_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    for (name, topology) in [
        (
            "matching but incomplete topology",
            PartialReplaceTopology {
                destination_second: false,
                source_second: false,
                temporary_second: false,
            },
        ),
        (
            "missing destination projection",
            PartialReplaceTopology { destination_second: false, ..EXACT_PARTIAL_REPLACE_TOPOLOGY },
        ),
        (
            "missing source projection",
            PartialReplaceTopology { source_second: false, ..EXACT_PARTIAL_REPLACE_TOPOLOGY },
        ),
        (
            "missing temporary projection",
            PartialReplaceTopology { temporary_second: false, ..EXACT_PARTIAL_REPLACE_TOPOLOGY },
        ),
    ] {
        let raw = partial_replace_program(
            &sources,
            &linear,
            &linux,
            1,
            raw::TypeId(3),
            PartialReplaceShape::Struct,
            topology,
        );
        let diagnostics =
            verify(raw, &sources, entry, linear.clone(), linux.clone()).expect_err(name);
        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.code() == "ZRYNA-I3010"
                    && (diagnostic.message().contains("exact sealed projection topology")
                        || diagnostic.message().contains("exact matching projection metadata"))
            }),
            "{name}: {diagnostics:?}"
        );
    }

    let mut extra = partial_replace_program(
        &sources,
        &linear,
        &linux,
        1,
        raw::TypeId(3),
        PartialReplaceShape::Struct,
        EXACT_PARTIAL_REPLACE_TOPOLOGY,
    );
    let function = &mut extra.modules[0].functions[0];
    let prepared = raw::PlaceId(7);
    push_partial_replace_place(
        &mut function.places,
        raw::TypeId(2),
        function.span,
        raw::PlaceKind::StructField { base: prepared, ordinal: 2 },
    );
    assert!(
        verify(extra, &sources, entry, linear.clone(), linux.clone())
            .expect_err("extra temporary topology")
            .iter()
            .any(|diagnostic| diagnostic.code() == "ZRYNA-I3006")
    );

    let mut extra_source = partial_replace_program(
        &sources,
        &linear,
        &linux,
        1,
        raw::TypeId(3),
        PartialReplaceShape::Struct,
        EXACT_PARTIAL_REPLACE_TOPOLOGY,
    );
    let function = &mut extra_source.modules[0].functions[0];
    push_partial_replace_place(
        &mut function.places,
        raw::TypeId(2),
        function.span,
        raw::PlaceKind::StructField { base: raw::PlaceId(3), ordinal: 2 },
    );
    assert!(
        verify(extra_source, &sources, entry, linear.clone(), linux.clone())
            .expect_err("extra source topology")
            .iter()
            .any(|diagnostic| diagnostic.code() == "ZRYNA-I3006")
    );

    let mut wrong = partial_replace_program(
        &sources,
        &linear,
        &linux,
        1,
        raw::TypeId(3),
        PartialReplaceShape::Struct,
        EXACT_PARTIAL_REPLACE_TOPOLOGY,
    );
    let function = &mut wrong.modules[0].functions[0];
    let raw::PlaceKind::StructField { ordinal, .. } = &mut function.places[9].kind else {
        panic!("prepared field")
    };
    *ordinal = 0;
    assert!(
        verify(wrong, &sources, entry, linear.clone(), linux.clone())
            .expect_err("wrong temporary topology")
            .iter()
            .any(|diagnostic| diagnostic.code() == "ZRYNA-I3006")
    );

    let mut wrong_destination = partial_replace_program(
        &sources,
        &linear,
        &linux,
        1,
        raw::TypeId(3),
        PartialReplaceShape::Struct,
        EXACT_PARTIAL_REPLACE_TOPOLOGY,
    );
    let function = &mut wrong_destination.modules[0].functions[0];
    let raw::PlaceKind::StructField { ordinal, .. } = &mut function.places[2].kind else {
        panic!("destination field")
    };
    *ordinal = 2;
    assert!(
        verify(wrong_destination, &sources, entry, linear, linux)
            .expect_err("wrong destination topology")
            .iter()
            .any(|diagnostic| diagnostic.code() == "ZRYNA-I3006")
    );
}

#[test]
fn root_replace_rejects_partial_destination_and_cleanup_owner_misuse() {
    let (sources, linear, linux) = pair_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let exact = partial_replace_program(
        &sources,
        &linear,
        &linux,
        1,
        raw::TypeId(3),
        PartialReplaceShape::Struct,
        EXACT_PARTIAL_REPLACE_TOPOLOGY,
    );

    let mut partial_destination = exact.clone();
    let function = &mut partial_destination.modules[0].functions[0];
    let moved_destination_leaf = push_partial_replace_place(
        &mut function.places,
        raw::TypeId(2),
        function.span,
        raw::PlaceKind::Temporary(raw::ValueId(5)),
    );
    function.blocks[0].instructions.insert(
        2,
        raw::Instruction {
            result: Some(raw::ValueDefinition {
                id: raw::ValueId(5),
                ty: raw::TypeId(2),
                span: function.span,
            }),
            span: function.span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(1) },
        },
    );
    function.cleanup_plans[0].actions.insert(0, raw::DropAction::DropPlace(moved_destination_leaf));
    let diagnostics = verify(partial_destination, &sources, entry, linear.clone(), linux.clone())
        .expect_err("partial destination");
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code() == "ZRYNA-I3010"
                && diagnostic.message().contains("replacement targets an unavailable")
        }),
        "{diagnostics:?}"
    );

    let mut stale_source_cleanup = exact.clone();
    stale_source_cleanup.modules[0].functions[0].cleanup_plans[0]
        .actions
        .insert(0, raw::DropAction::DropPlace(raw::PlaceId(3)));
    assert!(
        verify(stale_source_cleanup, &sources, entry, linear.clone(), linux.clone())
            .expect_err("cleanup includes transferred source")
            .iter()
            .any(|diagnostic| diagnostic.code() == "ZRYNA-I3012")
    );

    let mut dropped_destination_cleanup = exact;
    let function = &mut dropped_destination_cleanup.modules[0].functions[0];
    function.blocks[0].instructions.push(raw::Instruction {
        result: None,
        span: function.span,
        kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(0) },
    });
    assert!(
        verify(dropped_destination_cleanup, &sources, entry, linear, linux)
            .expect_err("cleanup includes dropped replacement destination")
            .iter()
            .any(|diagnostic| diagnostic.code() == "ZRYNA-I3012")
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn partial_replace_does_not_expand_enum_or_cfg_admission() {
    let (sources, linear, linux) = pair_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let mut partial_edge = partial_replace_program(
        &sources,
        &linear,
        &linux,
        1,
        raw::TypeId(3),
        PartialReplaceShape::Struct,
        EXACT_PARTIAL_REPLACE_TOPOLOGY,
    );
    let function = &mut partial_edge.modules[0].functions[0];
    function.blocks[0].instructions.pop().expect("remove replacement");
    let target_value = raw::ValueId(5);
    let target = push_partial_replace_place(
        &mut function.places,
        raw::TypeId(3),
        function.span,
        raw::PlaceKind::Temporary(target_value),
    );
    push_partial_replace_place(
        &mut function.places,
        raw::TypeId(2),
        function.span,
        raw::PlaceKind::StructField { base: target, ordinal: 0 },
    );
    push_partial_replace_place(
        &mut function.places,
        raw::TypeId(2),
        function.span,
        raw::PlaceKind::StructField { base: target, ordinal: 1 },
    );
    function.blocks[0].terminators[0].kind = raw::Terminator::Jump(raw::Edge {
        target: raw::BlockId(1),
        arguments: vec![raw::ValueId(4)],
    });
    function.blocks.push(raw::Block {
        id: raw::BlockId(1),
        parameters: vec![raw::ValueDefinition {
            id: target_value,
            ty: raw::TypeId(3),
            span: function.span,
        }],
        instructions: vec![],
        terminators: vec![raw::SpannedTerminator {
            span: function.span,
            kind: raw::Terminator::Return {
                value: raw::ValueId(2),
                cleanup: raw::CleanupPlanId(0),
            },
        }],
    });
    function.cleanup_plans[0].actions = vec![
        raw::DropAction::DropPlace(raw::PlaceId(6)),
        raw::DropAction::DropPlace(target),
        raw::DropAction::DropPlace(raw::PlaceId(0)),
    ];
    let diagnostics = verify(partial_edge, &sources, entry, linear, linux)
        .expect_err("partial CFG edge remains excluded");
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code() == "ZRYNA-I3010"
                && diagnostic.message().contains("CFG edge without mask transfer")
        }),
        "{diagnostics:?}"
    );

    let (sources, linear, linux) = payloadless_enum_authorities();
    let entry = sources.verify_file_id(0).expect("entry");
    let mut partial_enum = program(&sources, &linear, &linux);
    partial_enum.modules[0].data_declarations = 1;
    let function = &mut partial_enum.modules[0].functions[0];
    let span = function.span;
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(2), span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span },
    ];
    function.result = raw::TypeId(1);
    function.places = vec![
        raw::Place {
            id: raw::PlaceId(0),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Parameter(0),
        },
        raw::Place {
            id: raw::PlaceId(1),
            ty: raw::TypeId(3),
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(2)),
        },
        raw::Place {
            id: raw::PlaceId(2),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::EnumPayload { base: raw::PlaceId(1), variant: 1 },
        },
        raw::Place {
            id: raw::PlaceId(3),
            ty: raw::TypeId(3),
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(3)),
        },
        raw::Place {
            id: raw::PlaceId(4),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::EnumPayload { base: raw::PlaceId(3), variant: 1 },
        },
        raw::Place {
            id: raw::PlaceId(5),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(4)),
        },
        raw::Place {
            id: raw::PlaceId(6),
            ty: raw::TypeId(3),
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(5)),
        },
        raw::Place {
            id: raw::PlaceId(7),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::EnumPayload { base: raw::PlaceId(6), variant: 1 },
        },
    ];
    function.blocks[0].instructions = vec![
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(3), span }),
            span,
            kind: raw::InstructionKind::EnumConstruct { variant: 0, payload: None, cleanup: None },
        },
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(3), ty: raw::TypeId(3), span }),
            span,
            kind: raw::InstructionKind::EnumConstruct {
                variant: 1,
                payload: Some(raw::ValueId(0)),
                cleanup: None,
            },
        },
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(4), ty: raw::TypeId(2), span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(4) },
        },
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(5), ty: raw::TypeId(3), span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(3) },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::ReplacePlace {
                place: raw::PlaceId(1),
                value: raw::ValueId(5),
            },
        },
    ];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(1), cleanup: raw::CleanupPlanId(0) };
    function.cleanup_plans[0].actions = vec![
        raw::DropAction::DropPlace(raw::PlaceId(5)),
        raw::DropAction::DropPlace(raw::PlaceId(1)),
    ];
    let diagnostics = verify(partial_enum, &sources, entry, linear, linux)
        .expect_err("partial Enum replacement remains excluded");
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.code() == "ZRYNA-I3010"
                && diagnostic.message().contains("exact sealed projection topology")
        }),
        "{diagnostics:?}"
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn partial_aggregate_cannot_be_consumed_by_a_direct_call() {
    let (sources, linear, linux) = pair_authorities();
    let mut raw = program(&sources, &linear, &linux);
    raw.modules[0].data_declarations = 1;
    let span = raw.modules[0].functions[0].span;
    let caller = &mut raw.modules[0].functions[0];
    caller.entry_export = None;
    caller.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(3), span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span },
    ];
    caller.places = vec![
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
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(0), ordinal: 1 },
        },
        raw::Place {
            id: raw::PlaceId(3),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(2)),
        },
    ];
    caller.blocks[0].instructions = vec![
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(2), span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(1) },
        },
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(3), ty: raw::TypeId(1), span }),
            span,
            kind: raw::InstructionKind::DirectCall {
                callee: raw::FunctionId { module: raw::ModuleId(0), declaration: 1 },
                arguments: vec![raw::CallArgument::Value(raw::ValueId(0))],
                cleanup: raw::CleanupPlanId(0),
            },
        },
    ];
    caller.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(3), cleanup: raw::CleanupPlanId(1) };
    caller.cleanup_plans = vec![
        raw::CleanupPlan {
            id: raw::CleanupPlanId(0),
            span,
            actions: vec![
                raw::DropAction::DropPlace(raw::PlaceId(3)),
                raw::DropAction::DropPlace(raw::PlaceId(0)),
            ],
        },
        raw::CleanupPlan {
            id: raw::CleanupPlanId(1),
            span,
            actions: vec![
                raw::DropAction::DropPlace(raw::PlaceId(3)),
                raw::DropAction::DropPlace(raw::PlaceId(0)),
            ],
        },
    ];
    raw.modules[0].functions.push(raw::Function {
        id: raw::FunctionId { module: raw::ModuleId(0), declaration: 1 },
        entry_export: None,
        span,
        parameters: vec![raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(3), span }],
        borrow_parameters: vec![],
        result: raw::TypeId(1),
        places: vec![raw::Place {
            id: raw::PlaceId(0),
            ty: raw::TypeId(3),
            span,
            kind: raw::PlaceKind::Parameter(0),
        }],
        blocks: vec![raw::Block {
            id: raw::BlockId(0),
            parameters: vec![],
            instructions: vec![raw::Instruction {
                result: Some(raw::ValueDefinition {
                    id: raw::ValueId(1),
                    ty: raw::TypeId(1),
                    span,
                }),
                span,
                kind: raw::InstructionKind::I32Literal(0),
            }],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Return {
                    value: raw::ValueId(1),
                    cleanup: raw::CleanupPlanId(0),
                },
            }],
        }],
        cleanup_plans: vec![raw::CleanupPlan {
            id: raw::CleanupPlanId(0),
            span,
            actions: vec![raw::DropAction::DropPlace(raw::PlaceId(0))],
        }],
    });

    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("partial call");
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code() == "ZRYNA-I3010" && diagnostic.message().contains("without mask transfer")
    }));
}

fn partial_pair_return_program(
    sources: &SourceMap,
    linear: &zryna_layout::VerifiedLayouts,
    linux: &zryna_layout::VerifiedLayouts,
) -> raw::Program {
    let mut raw = program(sources, linear, linux);
    raw.modules[0].data_declarations = 1;
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(3), span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span },
    ];
    function.result = raw::TypeId(3);
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
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(0), ordinal: 1 },
        },
        raw::Place {
            id: raw::PlaceId(3),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(2)),
        },
        raw::Place {
            id: raw::PlaceId(4),
            ty: raw::TypeId(3),
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(3)),
        },
        raw::Place {
            id: raw::PlaceId(5),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(4), ordinal: 0 },
        },
        raw::Place {
            id: raw::PlaceId(6),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::StructField { base: raw::PlaceId(4), ordinal: 1 },
        },
    ];
    function.blocks[0].instructions = vec![
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(2), span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(1) },
        },
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(3), ty: raw::TypeId(3), span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(0) },
        },
    ];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(3), cleanup: raw::CleanupPlanId(0) };
    function.cleanup_plans[0].actions = vec![raw::DropAction::DropPlace(raw::PlaceId(3))];
    raw
}

#[test]
fn partial_temporary_return_requires_exact_topology_and_post_transfer_cleanup() {
    let (sources, linear, linux) = pair_authorities();
    let raw = partial_pair_return_program(&sources, &linear, &linux);
    let entry = sources.verify_file_id(0).expect("entry");
    let verified = verify(raw.clone(), &sources, entry, linear.clone(), linux.clone())
        .expect("exact-topology partial temporary return");
    let cleanup = verified
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
        .map(|action| action.root().index())
        .collect::<Vec<_>>();
    assert_eq!(cleanup, [3]);

    let mut missing = raw.clone();
    let function = &mut missing.modules[0].functions[0];
    function.places.remove(6);
    function.places.remove(2);
    for (index, place) in function.places.iter_mut().enumerate() {
        place.id = raw::PlaceId(u32::try_from(index).expect("bounded place"));
        if let raw::PlaceKind::StructField { base, .. } = &mut place.kind
            && *base == raw::PlaceId(4)
        {
            *base = raw::PlaceId(3);
        }
    }
    function.cleanup_plans[0].actions = vec![raw::DropAction::DropPlace(raw::PlaceId(2))];
    let diagnostics = verify(missing, &sources, entry, linear.clone(), linux.clone())
        .expect_err("missing return topology");
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code() == "ZRYNA-I3010"
            && diagnostic.message().contains("exact sealed projection topology")
    }));

    let mut extra = raw.clone();
    let function = &mut extra.modules[0].functions[0];
    function.places.push(raw::Place {
        id: raw::PlaceId(7),
        ty: raw::TypeId(2),
        span: function.span,
        kind: raw::PlaceKind::StructField { base: raw::PlaceId(4), ordinal: 2 },
    });
    assert!(
        verify(extra, &sources, entry, linear.clone(), linux.clone())
            .expect_err("extra return topology")
            .iter()
            .any(|diagnostic| diagnostic.code() == "ZRYNA-I3006")
    );

    let mut wrong = raw.clone();
    let function = &mut wrong.modules[0].functions[0];
    let raw::PlaceKind::StructField { ordinal, .. } = &mut function.places[6].kind else {
        panic!("field projection")
    };
    *ordinal = 2;
    assert!(
        verify(wrong, &sources, entry, linear.clone(), linux.clone())
            .expect_err("wrong return topology path")
            .iter()
            .any(|diagnostic| diagnostic.code() == "ZRYNA-I3006")
    );

    let mut returned_owner_cleanup = raw.clone();
    returned_owner_cleanup.modules[0].functions[0].cleanup_plans[0]
        .actions
        .insert(0, raw::DropAction::DropPlace(raw::PlaceId(4)));
    assert!(
        verify(returned_owner_cleanup, &sources, entry, linear.clone(), linux.clone(),)
            .expect_err("cleanup drops returned owner")
            .iter()
            .any(|diagnostic| diagnostic.code() == "ZRYNA-I3012")
    );

    let mut double_drop = raw;
    let function = &mut double_drop.modules[0].functions[0];
    function.blocks[0].instructions.push(raw::Instruction {
        result: None,
        span: function.span,
        kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(4) },
    });
    assert!(
        verify(double_drop, &sources, entry, linear, linux)
            .expect_err("returned owner was already dropped")
            .iter()
            .any(|diagnostic| diagnostic.code() == "ZRYNA-I3010")
    );
}

#[test]
fn partial_enum_temporary_cannot_be_returned() {
    let (sources, linear, linux) = payloadless_enum_authorities();
    let mut raw = program(&sources, &linear, &linux);
    raw.modules[0].data_declarations = 1;
    let function = &mut raw.modules[0].functions[0];
    let span = function.span;
    function.entry_export = None;
    function.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(3), span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span },
    ];
    function.result = raw::TypeId(3);
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
            kind: raw::PlaceKind::EnumPayload { base: raw::PlaceId(0), variant: 1 },
        },
        raw::Place {
            id: raw::PlaceId(2),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(2)),
        },
        raw::Place {
            id: raw::PlaceId(3),
            ty: raw::TypeId(3),
            span,
            kind: raw::PlaceKind::Temporary(raw::ValueId(3)),
        },
        raw::Place {
            id: raw::PlaceId(4),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::EnumPayload { base: raw::PlaceId(3), variant: 1 },
        },
    ];
    function.blocks[0].instructions = vec![
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(2), span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(1) },
        },
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(3), ty: raw::TypeId(3), span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(0) },
        },
    ];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(3), cleanup: raw::CleanupPlanId(0) };
    function.cleanup_plans[0].actions = vec![raw::DropAction::DropPlace(raw::PlaceId(2))];

    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("partial enum return");
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code() == "ZRYNA-I3010"
            && diagnostic.message().contains("exact sealed projection topology")
    }));
}

#[test]
fn direct_call_rejects_cleanup_that_drops_a_transferred_argument() {
    let (sources, linear, linux) = authorities();
    let mut raw = program(&sources, &linear, &linux);
    let span = raw.modules[0].functions[0].span;
    let caller = &mut raw.modules[0].functions[0];
    caller.entry_export = None;
    caller.parameters = vec![
        raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(2), span },
        raw::ValueDefinition { id: raw::ValueId(1), ty: raw::TypeId(1), span },
    ];
    caller.places = vec![raw::Place {
        id: raw::PlaceId(0),
        ty: raw::TypeId(2),
        span,
        kind: raw::PlaceKind::Parameter(0),
    }];
    caller.blocks[0].instructions = vec![raw::Instruction {
        result: Some(raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(1), span }),
        span,
        kind: raw::InstructionKind::DirectCall {
            callee: raw::FunctionId { module: raw::ModuleId(0), declaration: 1 },
            arguments: vec![raw::CallArgument::Value(raw::ValueId(0))],
            cleanup: raw::CleanupPlanId(0),
        },
    }];
    caller.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(2), cleanup: raw::CleanupPlanId(1) };
    caller.cleanup_plans = vec![
        raw::CleanupPlan {
            id: raw::CleanupPlanId(0),
            span,
            actions: vec![raw::DropAction::DropPlace(raw::PlaceId(0))],
        },
        raw::CleanupPlan { id: raw::CleanupPlanId(1), span, actions: vec![] },
    ];
    raw.modules[0].functions.push(raw::Function {
        id: raw::FunctionId { module: raw::ModuleId(0), declaration: 1 },
        entry_export: None,
        span,
        parameters: vec![raw::ValueDefinition { id: raw::ValueId(0), ty: raw::TypeId(2), span }],
        borrow_parameters: vec![],
        result: raw::TypeId(1),
        places: vec![raw::Place {
            id: raw::PlaceId(0),
            ty: raw::TypeId(2),
            span,
            kind: raw::PlaceKind::Parameter(0),
        }],
        blocks: vec![raw::Block {
            id: raw::BlockId(0),
            parameters: vec![],
            instructions: vec![raw::Instruction {
                result: Some(raw::ValueDefinition {
                    id: raw::ValueId(1),
                    ty: raw::TypeId(1),
                    span,
                }),
                span,
                kind: raw::InstructionKind::I32Literal(0),
            }],
            terminators: vec![raw::SpannedTerminator {
                span,
                kind: raw::Terminator::Return {
                    value: raw::ValueId(1),
                    cleanup: raw::CleanupPlanId(0),
                },
            }],
        }],
        cleanup_plans: vec![raw::CleanupPlan {
            id: raw::CleanupPlanId(0),
            span,
            actions: vec![raw::DropAction::DropPlace(raw::PlaceId(0))],
        }],
    });

    let entry = sources.verify_file_id(0).expect("entry");
    let diagnostics = verify(raw, &sources, entry, linear, linux).expect_err("call cleanup state");
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.code() == "ZRYNA-I3012"));
}

#[test]
fn explicit_drop_exposes_recursive_shape_from_its_pre_drop_state() {
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
    function.blocks[0].instructions = vec![
        raw::Instruction {
            result: Some(raw::ValueDefinition { id: raw::ValueId(2), ty: raw::TypeId(2), span }),
            span,
            kind: raw::InstructionKind::MoveFromPlace { place: raw::PlaceId(1) },
        },
        raw::Instruction {
            result: None,
            span,
            kind: raw::InstructionKind::DropPlace { place: raw::PlaceId(0) },
        },
    ];
    function.blocks[0].terminators[0].kind =
        raw::Terminator::Return { value: raw::ValueId(1), cleanup: raw::CleanupPlanId(0) };
    function.cleanup_plans[0].actions = vec![raw::DropAction::DropPlace(raw::PlaceId(2))];

    let entry = sources.verify_file_id(0).expect("entry");
    let verified = verify(raw, &sources, entry, linear, linux).expect("explicit recursive drop");
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
        .instructions()
        .nth(1)
        .expect("drop")
        .derived_drop_actions()
        .next()
        .expect("derived drop action");
    assert_eq!(action.root().index(), 0);
    assert_eq!(
        action.moved_projections().map(super::PlaceIdentity::index).collect::<Vec<_>>(),
        [1]
    );
    assert!(action.initialized_projections().next().is_none());
}
