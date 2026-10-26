//! Layout proof only. No parser packet, original-body proof or execution authority is issued.
use layout::raw::{
    Declaration, Field, Module, ModuleId, NodeId, NominalKind, TypeKind, TypeNode, Variant,
};
use zryna_layout::{StorageTarget, generic_v1 as layout};
use zryna_source::{SourceFileInput, SourceMap, UntrustedSpan};

pub(super) fn copy_enum() -> layout::VerifiedLayouts {
    const LEAF: &str = "interface Leaf extends ZrynaStruct { value:i32; }\n";
    const CHOICE: &str = "interface Choice extends ZrynaEnum { a:Leaf;b:Leaf;c:Leaf;d:Leaf;e:Leaf;f:Leaf;g:Leaf;h:Leaf; }";
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "main.zry".into(),
        text: format!("{LEAF}{CHOICE}"),
    }])
    .expect("source spans for isolated layout");
    let leaf_span = sources
        .verify_span(UntrustedSpan {
            file: 0,
            start: 0,
            end: u32::try_from(LEAF.len() - 1).expect("leaf span"),
        })
        .expect("leaf span");
    let choice_span = sources
        .verify_span(UntrustedSpan {
            file: 0,
            start: u32::try_from(LEAF.len()).expect("choice start"),
            end: u32::try_from(LEAF.len() + CHOICE.len()).expect("choice end"),
        })
        .expect("choice span");
    let primitive = |kind| TypeKind::Base(kind);
    let kinds = vec![
        primitive(zryna_layout::raw::TypeKind::Bool),
        primitive(zryna_layout::raw::TypeKind::I32),
        primitive(zryna_layout::raw::TypeKind::String),
        primitive(zryna_layout::raw::TypeKind::Struct {
            module: ModuleId(0),
            declaration: 0,
            fields: vec![Field { ordinal: 0, ty: NodeId(1) }],
        }),
        primitive(zryna_layout::raw::TypeKind::Enum {
            module: ModuleId(0),
            declaration: 1,
            variants: (0..8).map(|ordinal| Variant { ordinal, payload: Some(NodeId(3)) }).collect(),
        }),
    ];
    let graph = layout::raw::Graph {
        modules: vec![Module {
            id: ModuleId(0),
            source_file: sources.verify_file_id(0).expect("file"),
            data_declarations: 2,
        }],
        declarations: vec![
            Declaration {
                module: ModuleId(0),
                index: 0,
                kind: NominalKind::Struct,
                parameters: 0,
                members: 1,
                span: leaf_span,
            },
            Declaration {
                module: ModuleId(0),
                index: 1,
                kind: NominalKind::Enum,
                parameters: 0,
                members: 8,
                span: choice_span,
            },
        ],
        types: kinds
            .into_iter()
            .enumerate()
            .map(|(index, kind)| TypeNode {
                id: NodeId(u32::try_from(index).expect("node")),
                kind,
                span: match index {
                    3 => Some(leaf_span),
                    4 => Some(choice_span),
                    _ => None,
                },
            })
            .collect(),
        program_roots: (0..5).map(NodeId).collect(),
    };
    layout::verify(&graph, &sources, StorageTarget::Linear32V1)
        .expect("independent sealed layout only")
}
