use crate::data_ownership_v1::{RuntimeContractIdentity, raw};
use zryna_layout::{StorageTarget, raw as raw_layout};
use zryna_source::{SourceFileInput, SourceMap};

fn authorities() -> (SourceMap, zryna_layout::VerifiedLayouts, zryna_layout::VerifiedLayouts) {
    let text = "struct Envelope { value: String } export function relay(): i32 { return 0; }";
    let sources = SourceMap::build(vec![
        SourceFileInput { path: "00-main.zry".into(), text: text.into() },
        SourceFileInput { path: "01-lib.zry".into(), text: text.into() },
    ])
    .expect("source map");
    let main = sources.verify_file_id(0).expect("main source file");
    let library = sources.verify_file_id(1).expect("library source file");
    let graph = raw_layout::Graph {
        modules: vec![
            raw_layout::Module {
                id: raw_layout::ModuleId(0),
                source_file: main,
                data_declarations: 1,
            },
            raw_layout::Module {
                id: raw_layout::ModuleId(1),
                source_file: library,
                data_declarations: 0,
            },
        ],
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
                span: Some(sources.span(main, 0, 15).expect("nominal span")),
                kind: raw_layout::TypeKind::Struct {
                    module: raw_layout::ModuleId(0),
                    declaration: 0,
                    fields: vec![raw_layout::Field { ordinal: 0, ty: raw_layout::NodeId(2) }],
                },
            },
            raw_layout::TypeNode {
                id: raw_layout::NodeId(4),
                span: None,
                kind: raw_layout::TypeKind::Vec { element: raw_layout::NodeId(2) },
            },
            raw_layout::TypeNode {
                id: raw_layout::NodeId(5),
                span: None,
                kind: raw_layout::TypeKind::Shared { payload: raw_layout::NodeId(3) },
            },
            raw_layout::TypeNode {
                id: raw_layout::NodeId(6),
                span: None,
                kind: raw_layout::TypeKind::Weak { payload: raw_layout::NodeId(3) },
            },
        ],
        program_roots: vec![
            raw_layout::NodeId(3),
            raw_layout::NodeId(4),
            raw_layout::NodeId(5),
            raw_layout::NodeId(6),
        ],
    };
    let linear =
        zryna_layout::verify(&graph, &sources, StorageTarget::Linear32V1).expect("linear layouts");
    let linux = zryna_layout::verify(&graph, &sources, StorageTarget::LinuxX8664V1)
        .expect("native layouts");
    (sources, linear, linux)
}

#[allow(clippy::too_many_lines)]
pub(super) fn program()
-> (SourceMap, zryna_layout::VerifiedLayouts, zryna_layout::VerifiedLayouts, raw::Program) {
    let (sources, linear, linux) = authorities();
    let main = sources.verify_file_id(0).expect("main");
    let library = sources.verify_file_id(1).expect("library");
    let main_span = sources.span(main, 0, 75).expect("main span");
    let library_span = sources.span(library, 0, 75).expect("library span");
    let parameter_types = [3, 4, 5, 6, 2, 2];
    let definitions = |span| {
        parameter_types
            .iter()
            .enumerate()
            .map(|(id, ty)| raw::ValueDefinition {
                id: raw::ValueId(u32::try_from(id).expect("parameter id")),
                ty: raw::TypeId(*ty),
                span,
            })
            .collect::<Vec<_>>()
    };
    let places = |span| {
        parameter_types
            .iter()
            .enumerate()
            .map(|(id, ty)| raw::Place {
                id: raw::PlaceId(u32::try_from(id).expect("place id")),
                ty: raw::TypeId(*ty),
                span,
                kind: raw::PlaceKind::Parameter(u32::try_from(id).expect("parameter id")),
            })
            .collect::<Vec<_>>()
    };
    let callee_id = raw::FunctionId { module: raw::ModuleId(1), declaration: 0 };
    let mut caller_places = places(main_span);
    caller_places.push(raw::Place {
        id: raw::PlaceId(6),
        ty: raw::TypeId(3),
        span: main_span,
        kind: raw::PlaceKind::Temporary(raw::ValueId(6)),
    });
    let caller = raw::Function {
        id: raw::FunctionId { module: raw::ModuleId(0), declaration: 0 },
        entry_export: None,
        span: main_span,
        parameters: definitions(main_span),
        borrow_parameters: vec![],
        result: raw::TypeId(3),
        places: caller_places,
        blocks: vec![raw::Block {
            id: raw::BlockId(0),
            parameters: vec![],
            instructions: vec![raw::Instruction {
                result: Some(raw::ValueDefinition {
                    id: raw::ValueId(6),
                    ty: raw::TypeId(3),
                    span: main_span,
                }),
                span: main_span,
                kind: raw::InstructionKind::DirectCall {
                    callee: callee_id,
                    arguments: (0..6)
                        .map(|id| raw::CallArgument::Value(raw::ValueId(id)))
                        .collect(),
                    cleanup: raw::CleanupPlanId(0),
                },
            }],
            terminators: vec![raw::SpannedTerminator {
                span: main_span,
                kind: raw::Terminator::Return {
                    value: raw::ValueId(6),
                    cleanup: raw::CleanupPlanId(1),
                },
            }],
        }],
        cleanup_plans: vec![
            raw::CleanupPlan { id: raw::CleanupPlanId(0), span: main_span, actions: vec![] },
            raw::CleanupPlan { id: raw::CleanupPlanId(1), span: main_span, actions: vec![] },
        ],
    };
    let callee_function = raw::Function {
        id: callee_id,
        entry_export: None,
        span: library_span,
        parameters: definitions(library_span),
        borrow_parameters: vec![],
        result: raw::TypeId(3),
        places: places(library_span),
        blocks: vec![raw::Block {
            id: raw::BlockId(0),
            parameters: vec![],
            instructions: vec![],
            terminators: vec![raw::SpannedTerminator {
                span: library_span,
                kind: raw::Terminator::Return {
                    value: raw::ValueId(0),
                    cleanup: raw::CleanupPlanId(0),
                },
            }],
        }],
        cleanup_plans: vec![raw::CleanupPlan {
            id: raw::CleanupPlanId(0),
            span: library_span,
            actions: (1..6).rev().map(|id| raw::DropAction::DropPlace(raw::PlaceId(id))).collect(),
        }],
    };
    let raw = raw::Program {
        authorities: raw::AuthorityClaims {
            type_universe: linear.universe_identity().as_bytes(),
            linear32_fingerprint: *linear.fingerprint(),
            linux_x86_64_fingerprint: *linux.fingerprint(),
            runtime: RuntimeContractIdentity::OwnershipRuntimeV1,
        },
        entry_module: raw::ModuleId(0),
        modules: vec![
            raw::Module {
                id: raw::ModuleId(0),
                source_file: main,
                data_declarations: 1,
                functions: vec![caller],
            },
            raw::Module {
                id: raw::ModuleId(1),
                source_file: library,
                data_declarations: 0,
                functions: vec![callee_function],
            },
        ],
    };
    (sources, linear, linux, raw)
}
