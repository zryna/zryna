//! Independently literal same-source layout universes; no semantic output supplies expected roots.
use zryna_layout::{
    StorageTarget,
    generic_v1::{self as layout, raw},
};
use zryna_source::SourceMap;
use zryna_syntax::v5::VerifiedProjectSyntaxV5;

pub(super) fn graph(
    syntax: &VerifiedProjectSyntaxV5,
    sources: &SourceMap,
    attack: usize,
) -> raw::Graph {
    let spans = syntax.files()[0]
        .data_declarations
        .iter()
        .map(|data| sources.verify_span(data.span).expect("original declaration authority"))
        .collect::<Vec<_>>();
    assert_eq!(spans.len(), 3, "literal Plain, Box<T>, Foreign<T> declaration order");
    let mut kinds = vec![
        (None, raw::TypeKind::Base(zryna_layout::raw::TypeKind::Bool)),
        (None, raw::TypeKind::Base(zryna_layout::raw::TypeKind::I32)),
        (None, raw::TypeKind::Base(zryna_layout::raw::TypeKind::String)),
    ];
    if attack != 2 {
        kinds.push((
            Some(spans[0]),
            raw::TypeKind::Base(zryna_layout::raw::TypeKind::Struct {
                module: raw::ModuleId(0),
                declaration: 0,
                fields: vec![raw::Field { ordinal: 0, ty: raw::NodeId(1) }],
            }),
        ));
    }
    let declaration = if attack == 3 { 2 } else { 1 };
    kinds.push((
        Some(spans[declaration]),
        raw::TypeKind::Struct {
            module: raw::ModuleId(0),
            declaration: u32::try_from(declaration).expect("literal index"),
            arguments: vec![raw::NodeId(1)],
            fields: vec![raw::Field { ordinal: 0, ty: raw::NodeId(1) }],
        },
    ));
    if attack == 1 {
        kinds.push((
            Some(spans[1]),
            raw::TypeKind::Struct {
                module: raw::ModuleId(0),
                declaration: 1,
                arguments: vec![raw::NodeId(0)],
                fields: vec![raw::Field { ordinal: 0, ty: raw::NodeId(0) }],
            },
        ));
    }
    let count = u32::try_from(kinds.len()).expect("small independent graph");
    raw::Graph {
        modules: vec![raw::Module {
            id: raw::ModuleId(0),
            source_file: sources.verify_file_id(0).expect("original file"),
            data_declarations: 3,
        }],
        declarations: spans
            .into_iter()
            .enumerate()
            .map(|(index, span)| raw::Declaration {
                module: raw::ModuleId(0),
                index: u32::try_from(index).expect("index"),
                kind: raw::NominalKind::Struct,
                parameters: u32::from(index != 0),
                members: 1,
                span,
            })
            .collect(),
        types: kinds
            .into_iter()
            .enumerate()
            .map(|(index, (span, kind))| raw::TypeNode {
                id: raw::NodeId(u32::try_from(index).expect("id")),
                span,
                kind,
            })
            .collect(),
        program_roots: (0..count).map(raw::NodeId).collect(),
    }
}

pub(super) fn layouts(
    graph: &raw::Graph,
    sources: &SourceMap,
) -> (layout::VerifiedLayouts, layout::VerifiedLayouts) {
    (
        layout::verify(graph, sources, StorageTarget::Linear32V1)
            .expect("independent Linear32 same-source layout"),
        layout::verify(graph, sources, StorageTarget::LinuxX8664V1)
            .expect("independent Linux same-source layout"),
    )
}

/// A physically formed graph cannot bypass the independent mandatory nominal layout gate.
pub(super) fn missing_original_rejects_at_layout(graph: &raw::Graph, sources: &SourceMap) {
    for target in [StorageTarget::Linear32V1, StorageTarget::LinuxX8664V1] {
        let layout::Failure::Diagnostics(errors) = layout::verify(graph, sources, target)
            .expect_err("missing original cannot issue layouts")
        else {
            panic!("missing nominal requires structured layout rejection")
        };
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "ZRYNA-L3001");
        assert!(errors[0].primary_span().is_some());
    }
}
