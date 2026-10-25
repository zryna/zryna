//! Genuine borrowed-match source, checked layouts and actual private source producer work.
//! No raw test row constructs source permission or whole execution authority.
use super::{Failure, SourceMap, VerifiedProjectSyntaxV5, produce, v5};
use zryna_layout::{StorageTarget, generic_v1 as layout};
use zryna_source::SourceFileInput;
fn project(provider: &str) -> (SourceMap, VerifiedProjectSyntaxV5) {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/m7-generic-execution-borrowed-match");
    let text = std::fs::read_to_string(directory.join("literals/option-shared-both.zry"))
        .expect("unchanged source frozen before actual providers");
    let packet = std::fs::read(directory.join(format!("option-shared-both-{provider}.json")))
        .expect("fresh actual full provider packet");
    let sources = SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text }])
        .expect("real original source map");
    let syntax =
        v5::verify_snapshot(v5::decode_snapshot(&packet).expect("strict raw v5 packet"), &sources)
            .expect("genuine original syntax authority");
    (sources, syntax)
}
fn graph(sources: &SourceMap) -> layout::raw::Graph {
    use zryna_layout::raw::TypeKind as Base;
    let shapes = [
        layout::raw::TypeKind::Base(Base::Bool),
        layout::raw::TypeKind::Base(Base::I32),
        layout::raw::TypeKind::Base(Base::String),
        layout::raw::TypeKind::Option { argument: layout::raw::NodeId(2) },
    ];
    let types = shapes
        .into_iter()
        .enumerate()
        .map(|(id, kind)| layout::raw::TypeNode {
            id: layout::raw::NodeId(u32::try_from(id).expect("four exact types")),
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
        program_roots: types.iter().map(|node| node.id).collect(),
        types,
    }
}
#[test]
fn actual_borrowed_match_replay_and_retained_facts_exact_extra_overflow_and_retry_are_atomic() {
    for provider in ["native", "typescript"] {
        let (sources, syntax) = project(provider);
        let graph = graph(&sources);
        let linear = layout::verify(&graph, &sources, StorageTarget::Linear32V1)
            .expect("independent exact Option<String> layout");
        let linux = layout::verify(&graph, &sources, StorageTarget::LinuxX8664V1)
            .expect("independent exact Linux Option<String> layout");
        let entry = sources.verify_file_id(0).expect("genuine entry");
        let run =
            |spent: &mut usize| produce(&syntax, &sources, entry, &linear, &linux, &[], spent);
        let mut cost = 0;
        let (expected, _) = run(&mut cost).expect("actual original-to-borrowed-match producer");
        assert!(cost > 0);
        let limit = crate::generic_v1::execution_v1::budget::MAX_OWNER_UNITS;
        let mut exact = limit - cost;
        assert_eq!(run(&mut exact).expect("exact credit").0, expected);
        assert_eq!(exact, limit);
        for mut short in [limit - cost + 1, usize::MAX] {
            let before = short;
            assert!(matches!(run(&mut short), Err(Failure::Diagnostics(_))));
            assert_eq!(short, before, "failed complete producer keeps caller ledger atomic");
        }
        let mut retry = 0;
        assert_eq!(run(&mut retry).expect("pristine actual source replay").0, expected);
        assert_eq!(retry, cost);
    }
}
