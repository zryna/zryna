//! Complete source-authenticated packets and independently authored layouts.
use layout::raw::{Declaration, Field, Module, ModuleId, NodeId, NominalKind, TypeKind, TypeNode};
use zryna_layout::{StorageTarget, generic_v1 as layout};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5::{self, VerifiedProjectSyntaxV5};

pub(super) struct Fixture {
    name: &'static str,
    generic: bool,
    pub(super) sources: SourceMap,
    pub(super) syntax: VerifiedProjectSyntaxV5,
    pub(super) graph: layout::raw::Graph,
}
impl Fixture {
    pub(super) fn new(name: &'static str, generic: bool) -> Self {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/m7-generic-struct-v4");
        let source = match name {
            "construction" => include_str!("../../../../tests/m7-generic-struct-v4/construction.zry").to_owned(),
            "pass-generic" => "interface Box<T extends ZrynaValue> extends ZrynaStruct { value: T; } function pass(value: Box<i32>): Box<i32> { return value; }".to_owned(),
            "pass-plain" => "interface Plain extends ZrynaStruct { value: i32; } function pass(value: Plain): Plain { return value; }".to_owned(),
            _ => panic!("unknown independent source fixture"),
        };
        let sources =
            SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: source }])
                .expect("source authority");
        let packet = std::fs::read(path.join(format!("{name}-reference.json")))
            .expect("untrusted v5 packet");
        let syntax =
            v5::verify_snapshot(v5::decode_snapshot(&packet).expect("v5 decoding"), &sources)
                .expect("original source authentication");
        let span = sources
            .verify_span(syntax.files()[0].data_declarations[0].span)
            .expect("original declaration");
        let fields = vec![Field { ordinal: 0, ty: NodeId(1) }];
        let nominal = if generic {
            TypeKind::Struct {
                module: ModuleId(0),
                declaration: 0,
                arguments: vec![NodeId(1)],
                fields,
            }
        } else {
            TypeKind::Base(zryna_layout::raw::TypeKind::Struct {
                module: ModuleId(0),
                declaration: 0,
                fields,
            })
        };
        let kinds = vec![
            TypeKind::Base(zryna_layout::raw::TypeKind::Bool),
            TypeKind::Base(zryna_layout::raw::TypeKind::I32),
            TypeKind::Base(zryna_layout::raw::TypeKind::String),
            nominal,
        ];
        let graph = layout::raw::Graph {
            modules: vec![Module {
                id: ModuleId(0),
                source_file: sources.verify_file_id(0).expect("entry"),
                data_declarations: 1,
            }],
            declarations: vec![Declaration {
                module: ModuleId(0),
                index: 0,
                kind: NominalKind::Struct,
                parameters: u32::from(generic),
                members: 1,
                span,
            }],
            types: kinds
                .into_iter()
                .enumerate()
                .map(|(i, kind)| TypeNode {
                    id: NodeId(u32::try_from(i).expect("id")),
                    span: (i == 3).then_some(span),
                    kind,
                })
                .collect(),
            program_roots: (0..4).map(NodeId).collect(),
        };
        Self { name, generic, sources, syntax, graph }
    }
    pub(super) fn fresh(&self) -> Self {
        Self::new(self.name, self.generic)
    }
    pub(super) fn layouts(&self) -> (layout::VerifiedLayouts, layout::VerifiedLayouts) {
        (
            layout::verify(&self.graph, &self.sources, StorageTarget::Linear32V1)
                .expect("independent linear layout"),
            layout::verify(&self.graph, &self.sources, StorageTarget::LinuxX8664V1)
                .expect("independent linux layout"),
        )
    }
}
