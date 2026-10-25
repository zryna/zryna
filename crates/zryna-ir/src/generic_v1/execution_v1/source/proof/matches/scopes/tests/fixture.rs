//! Actual frozen provider DTO and independent five-type layouts; no fabricated source proof.
use crate::generic_v1::execution_v1::{raw, source};
use zryna_layout::{StorageTarget, generic_v1 as layout};
use zryna_source::{SourceFileInput, SourceMap};

pub(super) fn program(
    provider: &str,
    check: impl FnOnce(
        &raw::Program,
        &source::SourceProof,
        &layout::VerifiedLayouts,
        &layout::VerifiedLayouts,
    ),
) {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/m7-generic-execution-indexed-borrowed-match");
    let case = "indexed-option-exclusive-join";
    let text = std::fs::read_to_string(directory.join(format!("literals/{case}.zry")))
        .expect("unchanged source fixture");
    let packet = std::fs::read(directory.join(format!("{case}-{provider}.json")))
        .expect("actual provider snapshot");
    let sources = SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text }])
        .expect("genuine source map");
    let syntax = zryna_syntax::v5::verify_snapshot(
        zryna_syntax::v5::decode_snapshot(&packet).expect("strict full provider DTO"),
        &sources,
    )
    .expect("authenticated original source");
    let graph = graph(&sources);
    let linear = layout::verify(&graph, &sources, StorageTarget::Linear32V1)
        .expect("independent linear layout");
    let linux = layout::verify(&graph, &sources, StorageTarget::LinuxX8664V1)
        .expect("independent native layout");
    let entry = sources.verify_file_id(0).expect("source entry");
    let (program, proof) = source::produce(&syntax, &sources, entry, &linear, &linux, &[], &mut 0)
        .expect("genuine source replay and private original proof");
    check(&program, &proof, &linear, &linux);
}

fn graph(sources: &SourceMap) -> layout::raw::Graph {
    use zryna_layout::raw::TypeKind as Base;
    let shapes = [
        layout::raw::TypeKind::Base(Base::Bool),
        layout::raw::TypeKind::Base(Base::I32),
        layout::raw::TypeKind::Base(Base::String),
        layout::raw::TypeKind::Option { argument: layout::raw::NodeId(2) },
        layout::raw::TypeKind::Base(Base::Vec { element: layout::raw::NodeId(3) }),
    ];
    let types = shapes
        .into_iter()
        .enumerate()
        .map(|(id, kind)| layout::raw::TypeNode {
            id: layout::raw::NodeId(u32::try_from(id).expect("five actual types")),
            span: None,
            kind,
        })
        .collect::<Vec<_>>();
    layout::raw::Graph {
        modules: vec![layout::raw::Module {
            id: layout::raw::ModuleId(0),
            source_file: sources.verify_file_id(0).expect("entry"),
            data_declarations: 0,
        }],
        declarations: Vec::new(),
        program_roots: types.iter().map(|n| n.id).collect(),
        types,
    }
}

pub(super) fn match_site(p: &raw::Program) -> (usize, usize) {
    p.functions
        .iter()
        .enumerate()
        .find_map(|(f, function)| {
            function.blocks.iter().enumerate().find_map(|(b, block)| {
                matches!(block.terminator, raw::Terminator::ClosedEnumMatch { .. })
                    .then_some((f, b))
            })
        })
        .expect("genuine borrowed match source")
}
