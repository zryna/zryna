//! Genuine source amplification: lexical state copied by match must remain bounded.
use crate::generic_v1::{Failure, owned_v2};
use zryna_layout::{StorageTarget, generic_v1 as layout};
use zryna_source::SourceMap;

mod authenticated;
use authenticated::authenticated;

fn layouts(sources: &SourceMap) -> (layout::VerifiedLayouts, layout::VerifiedLayouts) {
    use layout::raw::{Graph, Module, ModuleId, NodeId, TypeKind, TypeNode};
    let graph = Graph {
        modules: vec![Module {
            id: ModuleId(0),
            source_file: sources.verify_file_id(0).expect("file"),
            data_declarations: 0,
        }],
        declarations: vec![],
        types: vec![
            TypeNode {
                id: NodeId(0),
                span: None,
                kind: TypeKind::Base(zryna_layout::raw::TypeKind::Bool),
            },
            TypeNode {
                id: NodeId(1),
                span: None,
                kind: TypeKind::Base(zryna_layout::raw::TypeKind::I32),
            },
            TypeNode {
                id: NodeId(2),
                span: None,
                kind: TypeKind::Base(zryna_layout::raw::TypeKind::String),
            },
            TypeNode { id: NodeId(3), span: None, kind: TypeKind::Option { argument: NodeId(1) } },
        ],
        program_roots: vec![NodeId(3)],
    };
    (
        layout::verify(&graph, sources, StorageTarget::Linear32V1)
            .expect("independent Linear32 layout"),
        layout::verify(&graph, sources, StorageTarget::LinuxX8664V1)
            .expect("independent Linux layout"),
    )
}

fn accepted_source_wire() -> Vec<u8> {
    let (sources, syntax) = authenticated(1);
    let (linear, linux) = layouts(&sources);
    let claim = owned_v2::produce_claim(&syntax, &sources, &linear, &linux, &[])
        .expect("small source crosses the public producer");
    let bytes = owned_v2::wire::encode(&claim).expect("bounded public wire");
    let runtime = zryna_ownership_runtime_abi::generic_v1::verify_v1(
        zryna_ownership_runtime_abi::generic_v1::raw_v1(&linear, &linux).expect("runtime claims"),
        &linear,
        &linux,
    )
    .expect("runtime declaration authority");
    owned_v2::verify(
        owned_v2::wire::decode(&bytes).expect("public hostile-wire decoder"),
        &syntax,
        &sources,
        sources.verify_file_id(0).expect("entry"),
        &linear,
        &linux,
        &runtime,
    )
    .expect("small source independently seals its complete type/owner/cleanup claim");
    bytes
}

#[test]
fn repeated_match_binding_copies_exceed_source_state_budget_before_claim_sealing() {
    let pristine = accepted_source_wire();
    // Each match saves 400 scalar bindings and restores them into both arms. Counting only
    // those three copies, each binding and its one-byte type key already costs 2 units:
    // 440 * 3 * 400 * 2 = 1,056,000 > the unchanged 1,048,576-state-unit ceiling.
    // Availability vectors, the enum parameter, loans and merge state add further work.
    // These 440 matches remain below independent source/IR value, block and wire ceilings.
    for _ in 0..2 {
        let (sources, syntax) = authenticated(440);
        let (linear, linux) = layouts(&sources);
        let Err(failure) = owned_v2::produce_claim(&syntax, &sources, &linear, &linux, &[]) else {
            panic!("match snapshots published an over-budget raw claim");
        };
        let Failure::Diagnostics(errors) = failure else {
            panic!("expected deterministic source-state budget rejection");
        };
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code(), "ZRYNA-I3201");
        assert_eq!(errors[0].message(), "owned source specialization aggregate ceiling");
    }
    assert_eq!(accepted_source_wire(), pristine, "rejection must leave pristine replay unchanged");
}
