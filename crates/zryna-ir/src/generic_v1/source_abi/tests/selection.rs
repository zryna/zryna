//! An imported aggregate helper and source-visible template do not become host exports.
use crate::generic_v1::{Failure, owned_v2};
use zryna_layout::{StorageTarget, generic_v1 as layout};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5;

fn check(aggregate: bool) -> Result<(), Failure> {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/m7-generic-source-abi");
    let name = if aggregate { "aggregate-entry" } else { "scalar-entry" };
    let texts = ["a", "z"].map(|part| {
        std::fs::read_to_string(root.join(format!("{name}-{part}.zry"))).expect("literal source")
    });
    let sources = SourceMap::build(vec![
        SourceFileInput { path: "a.zry".into(), text: texts[0].clone() },
        SourceFileInput { path: "z.zry".into(), text: texts[1].clone() },
    ])
    .expect("literal two-module sources");
    let syntax = v5::verify_snapshot(
        v5::decode_snapshot(
            &std::fs::read(root.join(format!("{name}.json"))).expect("literal DTO"),
        )
        .expect("untrusted DTO decoding"),
        &sources,
    )
    .expect("complete independent literal source authentication");
    let nodes = [
        zryna_layout::raw::TypeKind::Bool,
        zryna_layout::raw::TypeKind::I32,
        zryna_layout::raw::TypeKind::String,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, kind)| layout::raw::TypeNode {
        id: layout::raw::NodeId(index.try_into().expect("primitive")),
        span: None,
        kind: layout::raw::TypeKind::Base(kind),
    })
    .collect::<Vec<_>>();
    let graph = layout::raw::Graph {
        modules: [0, 1]
            .map(|id| layout::raw::Module {
                id: layout::raw::ModuleId(id),
                source_file: sources.verify_file_id(id).expect("file"),
                data_declarations: 0,
            })
            .into(),
        declarations: vec![],
        program_roots: nodes.iter().map(|node| node.id).collect(),
        types: nodes,
    };
    let linear =
        layout::verify(&graph, &sources, StorageTarget::Linear32V1).expect("linear layouts");
    let linux =
        layout::verify(&graph, &sources, StorageTarget::LinuxX8664V1).expect("linux layouts");
    let claim = owned_v2::produce_claim(&syntax, &sources, &linear, &linux, &[])?;
    let runtime = zryna_ownership_runtime_abi::generic_v1::verify_v1(
        zryna_ownership_runtime_abi::generic_v1::raw_v1(&linear, &linux).expect("runtime claims"),
        &linear,
        &linux,
    )
    .expect("runtime authority");
    let result = owned_v2::verify(
        owned_v2::wire::decode(&owned_v2::wire::encode(&claim)?)?,
        &syntax,
        &sources,
        sources.verify_file_id(1).expect("nonzero selected entry"),
        &linear,
        &linux,
        &runtime,
    );
    if aggregate {
        let Failure::Diagnostics(errors) = result.expect_err("aggregate entry is not scalar")
        else {
            panic!("source diagnostic");
        };
        assert_eq!(errors[0].code(), "ZRYNA-M7005");
        let at = errors[0].primary_span().expect("original exported function name");
        assert_eq!(at.file(), sources.verify_file_id(1).expect("entry file"));
        let start = texts[1].find("value").expect("entry name");
        assert_eq!((at.start() as usize, at.end() as usize), (start, start + 5));
    } else {
        let program = result?;
        assert_eq!(program.scalar_abi().exports().len(), 1);
        assert_eq!(program.export_functions().len(), 1);
        assert_eq!(
            program.functions().len(),
            2,
            "unused generic template emits no executable instance"
        );
    }
    Ok(())
}

#[test]
fn nonzero_scalar_entry_ignores_imported_aggregate_export_and_generic_template() {
    check(false).expect("imported aggregate helper and template remain internal");
}

#[test]
fn nonzero_aggregate_entry_reports_its_own_original_export_name() {
    check(true).expect("M7005 remains bound to the selected entry name");
}
