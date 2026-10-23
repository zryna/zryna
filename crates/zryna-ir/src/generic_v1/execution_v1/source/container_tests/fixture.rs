//! Genuine packets and independently verified fixture layouts, with no execution proof constructor.
use super::super::{SourceMap, VerifiedLayouts, VerifiedProjectSyntaxV5, produce_claim, raw, v5};
use zryna_layout::{StorageTarget, generic_v1 as layout};
use zryna_source::SourceFileInput;
const GROUP: &str = "m7-generic-execution-containers";
pub(super) fn project(case: &str, provider: &str) -> (SourceMap, VerifiedProjectSyntaxV5) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests").join(GROUP);
    let text = std::fs::read_to_string(path.join("literals").join(format!("{case}.zry")))
        .expect("pre-provider frozen literal");
    let bytes = std::fs::read(path.join(format!("{case}-{provider}.json")))
        .expect("actual fresh named provider output");
    let sources = SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text }])
        .expect("literal source identity");
    let syntax =
        v5::verify_snapshot(v5::decode_snapshot(&bytes).expect("strict complete DTO"), &sources)
            .expect("owning source syntax authentication");
    (sources, syntax)
}
fn layouts(sources: &SourceMap, case: &str) -> (VerifiedLayouts, VerifiedLayouts) {
    use zryna_layout::raw::TypeKind as Base;
    let mut shapes = vec![Base::Bool, Base::I32, Base::String];
    match case {
        "vec-copy" => shapes.push(Base::Vec { element: layout::raw::NodeId(1) }),
        "vec-owned-prepare" => shapes.push(Base::Vec { element: layout::raw::NodeId(2) }),
        "shared-weak-count" => {
            shapes.push(Base::Shared { payload: layout::raw::NodeId(2) });
            shapes.push(Base::Weak { payload: layout::raw::NodeId(2) });
        }
        "opaque-count-unused" => {}
        _ => panic!("only the four settled source cases get layouts"),
    }
    let types = shapes
        .into_iter()
        .enumerate()
        .map(|(id, kind)| layout::raw::TypeNode {
            id: layout::raw::NodeId(u32::try_from(id).expect("small independent inventory")),
            span: None,
            kind: layout::raw::TypeKind::Base(kind),
        })
        .collect::<Vec<_>>();
    let graph = layout::raw::Graph {
        modules: vec![layout::raw::Module {
            id: layout::raw::ModuleId(0),
            source_file: sources.verify_file_id(0).expect("module"),
            data_declarations: 0,
        }],
        declarations: vec![],
        program_roots: types.iter().map(|node| node.id).collect(),
        types,
    };
    (
        layout::verify(&graph, sources, StorageTarget::Linear32V1)
            .expect("independent linear layout"),
        layout::verify(&graph, sources, StorageTarget::LinuxX8664V1)
            .expect("independent native layout"),
    )
}
pub(super) fn keys(case: &str) -> Vec<Vec<u8>> {
    if case != "vec-copy" {
        return vec![];
    }
    // Actual original pack<i32>: module0/function0/arity1/argument i32.
    let mut key = vec![0x40];
    for lane in [0u32, 0, 1, 1] {
        key.extend_from_slice(&lane.to_le_bytes());
    }
    key.push(1);
    vec![key]
}
pub(super) fn claims(
    case: &str,
    provider: &str,
) -> (SourceMap, VerifiedProjectSyntaxV5, VerifiedLayouts, VerifiedLayouts, raw::Program) {
    let (sources, syntax) = project(case, provider);
    let (linear, linux) = layouts(&sources, case);
    let keys = keys(case);
    let borrowed = keys.iter().map(Vec::as_slice).collect::<Vec<_>>();
    let claim = produce_claim(
        &syntax,
        &sources,
        sources.verify_file_id(0).expect("entry"),
        &linear,
        &linux,
        &borrowed,
    )
    .expect("genuine original-to-untrusted source replay only");
    (sources, syntax, linear, linux, claim)
}
