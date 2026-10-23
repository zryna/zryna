//! Independent literal prefix-failure plan; no source producer or plan oracle is called.
use super::*;
use zryna_layout::{StorageTarget, generic_v1 as layout};
use zryna_source::{SourceFileInput, SourceMap, UntrustedSpan};
const AT: UntrustedSpan = UntrustedSpan { file: 0, start: 0, end: 1 };

fn layouts() -> VerifiedLayouts {
    use layout::raw::{
        Declaration, Field, Graph, Module, ModuleId, NodeId, NominalKind, TypeKind, TypeNode,
    };
    let sources =
        SourceMap::build(vec![SourceFileInput { path: "raw.zry".into(), text: "x".into() }])
            .expect("span authority only, no original source seal claimed");
    let span = sources.verify_span(AT).expect("span");
    let kinds = vec![
        TypeKind::Base(zryna_layout::raw::TypeKind::Bool),
        TypeKind::Base(zryna_layout::raw::TypeKind::I32),
        TypeKind::Base(zryna_layout::raw::TypeKind::String),
        TypeKind::Struct {
            module: ModuleId(0),
            declaration: 0,
            arguments: vec![NodeId(2)],
            fields: vec![Field { ordinal: 0, ty: NodeId(2) }, Field { ordinal: 1, ty: NodeId(2) }],
        },
    ];
    layout::verify(
        &Graph {
            modules: vec![Module {
                id: ModuleId(0),
                source_file: sources.verify_file_id(0).expect("file"),
                data_declarations: 1,
            }],
            declarations: vec![Declaration {
                module: ModuleId(0),
                index: 0,
                kind: NominalKind::Struct,
                parameters: 1,
                members: 2,
                span,
            }],
            types: kinds
                .into_iter()
                .enumerate()
                .map(|(index, kind)| TypeNode {
                    id: NodeId(u32::try_from(index).expect("small")),
                    span: (index == 3).then_some(span),
                    kind,
                })
                .collect(),
            program_roots: (0..4).map(NodeId).collect(),
        },
        &sources,
        StorageTarget::Linear32V1,
    )
    .expect("independent sealed field/drop inventory")
}
fn claim() -> (graph::Program, Vec<Vec<raw::Extension>>) {
    let types = [2, 2, 2, 3, 1];
    let instructions = types
        .into_iter()
        .enumerate()
        .map(|(id, ty)| graph::Instruction {
            result: graph::Definition {
                id: u32::try_from(id).expect("small"),
                ty: graph::Type::Stored(ty),
            },
            span: AT,
            operation: if id == 4 {
                graph::Operation::I32Literal(7)
            } else {
                graph::Operation::Unit
            },
        })
        .collect();
    let operations = vec![
        raw::Operation::StringLiteral(b"earlier-root".to_vec()),
        raw::Operation::StringLiteral(b"left".to_vec()),
        raw::Operation::CloneString(0),
        raw::Operation::StructConstruct { ty: 3, fields: vec![1, 2] },
    ];
    (
        graph::Program {
            modules: vec![],
            declarations: vec![],
            type_keys: vec![],
            universe: [0; 32],
            linear32: [0; 32],
            linux_x86_64: [0; 32],
            functions: vec![graph::Function {
                key: vec![0x41, 0, 0, 0, 0, 0, 0, 0, 0],
                span: AT,
                public_export: None,
                parameters: vec![],
                result: graph::Type::Stored(1),
                blocks: vec![graph::Block {
                    id: 0,
                    span: AT,
                    parameters: vec![],
                    instructions,
                    terminator: graph::Terminator::Return(4),
                }],
            }],
        },
        vec![
            operations
                .into_iter()
                .enumerate()
                .map(|(id, operation)| raw::Extension {
                    result: u32::try_from(id).expect("small"),
                    operation,
                })
                .collect(),
        ],
    )
}
#[test]
fn literal_struct_prefix_cleanup_consumes_only_fully_initialized_fields() {
    let layout = layouts();
    let (graph, extensions) = claim();
    let expected = vec![raw::Plan {
        steps: vec![
            raw::Step { block: 0, position: 0, end_loans: vec![], cleanup: vec![], failure: true },
            raw::Step { block: 0, position: 1, end_loans: vec![], cleanup: vec![0], failure: true },
            raw::Step {
                block: 0,
                position: 2,
                end_loans: vec![],
                cleanup: vec![1, 0],
                failure: true,
            },
            raw::Step {
                block: 0,
                position: 5,
                end_loans: vec![],
                cleanup: vec![3, 0],
                failure: false,
            },
        ],
    }];
    assert_eq!(derive(&graph, &extensions, &layout).expect("literal prefix"), expected);
    let mut duplicate = extensions.clone();
    duplicate[0][3].operation = raw::Operation::StructConstruct { ty: 3, fields: vec![1, 1] };
    assert!(derive(&graph, &duplicate, &layout).is_err(), "same affine field cannot move twice");
    let mut unavailable = extensions.clone();
    unavailable[0][3].operation = raw::Operation::StructConstruct { ty: 3, fields: vec![1, 99] };
    assert!(derive(&graph, &unavailable, &layout).is_err(), "uninitialized suffix is not an owner");
    assert_eq!(derive(&graph, &extensions, &layout).expect("fresh replay"), expected);
}
