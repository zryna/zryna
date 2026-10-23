//! Exact named packets and semantic-demand-owned layouts; no fabricated execution proof.
use crate::bounded_generics_v1::{
    SemanticInput,
    body_types::check_body_types,
    instantiation::{execution_v1, layouts::verify_layouts},
    resolve_declarations,
};
use sha2::{Digest, Sha256};
use std::path::Path;
use zryna_ir::generic_v1::execution_v1::raw;
use zryna_layout::{StorageTarget, generic_v1::VerifiedLayouts};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5::{VerifiedProjectSyntaxV5, decode_snapshot, verify_snapshot};
pub(super) const SETTLED: [&str; 4] =
    ["vec-copy", "vec-owned-prepare", "shared-weak-count", "opaque-count-unused"];
pub(super) const PROVIDERS: [&str; 2] = ["native", "typescript"];
fn read(name: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/m7-generic-execution-containers")
            .join(name),
    )
    .expect("unchanged frozen container input")
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn pinned(name: &str) -> Vec<u8> {
    let identities: serde_json::Value = serde_json::from_str(include_str!("input-identities.json"))
        .expect("pre-test frozen identities");
    let bytes = read(name);
    assert_eq!(
        u64::try_from(bytes.len()).expect("bounded input"),
        identities[name]["bytes"].as_u64().expect("frozen count")
    );
    assert_eq!(digest(&bytes), identities[name]["sha256"]);
    bytes
}
pub(super) fn expected() -> serde_json::Value {
    serde_json::from_slice(&pinned("SOURCE_EXPECTED_FREEZE.json"))
        .expect("unchanged original source expectations")
}
pub(super) fn row(case: &str) -> serde_json::Value {
    expected()["cases"]
        .as_array()
        .expect("nine frozen literals")
        .iter()
        .find(|row| row["case"] == case)
        .expect("frozen source case")
        .clone()
}
pub(super) fn project(case: &str, provider: &str) -> (SourceMap, VerifiedProjectSyntaxV5) {
    assert!(PROVIDERS.contains(&provider));
    let source = read(&format!("literals/{case}.zry"));
    let expected = row(case);
    assert_eq!(digest(&source), expected["sha256"]);
    let identities: serde_json::Value = serde_json::from_slice(&pinned("packet-identities.json"))
        .expect("actual fresh process identities");
    let record = identities["records"]
        .as_array()
        .expect("eighteen named packets")
        .iter()
        .find(|row| row["case"] == case && row["provider"] == provider)
        .expect("honest provider identity");
    assert_eq!(record["exactSourceSha256"], expected["sha256"]);
    assert_eq!(record["acceptedSyntaxWithoutDiagnostics"], true);
    let packet = read(record["packet"].as_str().expect("actual packet path"));
    assert_eq!(digest(&packet), record["packetSha256"]);
    assert_eq!(
        u64::try_from(packet.len()).expect("bounded packet"),
        record["packetBytes"].as_u64().expect("exact process output bytes")
    );
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "main.zry".into(),
        text: String::from_utf8(source).expect("frozen UTF8"),
    }])
    .expect("actual original source identity");
    let syntax =
        verify_snapshot(decode_snapshot(&packet).expect("complete strict v5 DTO"), &sources)
            .expect("exact original syntax authentication");
    (sources, syntax)
}
pub(super) fn with_claim(
    case: &str,
    provider: &str,
    inspect: impl FnOnce(
        &SourceMap,
        &VerifiedProjectSyntaxV5,
        &VerifiedLayouts,
        &VerifiedLayouts,
        &raw::Program,
    ),
) {
    assert!(SETTLED.contains(&case));
    let (sources, syntax) = project(case, provider);
    let entry = sources.verify_file_id(0).expect("original main entry");
    let declarations = resolve_declarations(
        SemanticInput::try_new(&syntax, &sources, entry).expect("source-bound semantic input"),
    )
    .expect("all original declarations");
    let bodies = check_body_types(&declarations).expect("all original capabilities and ownership");
    let instances = execution_v1::discover(&bodies).expect("new original-before-generated bridge");
    let linear = verify_layouts(instances.instances(), StorageTarget::Linear32V1)
        .expect("actual semantic-demand linear layouts");
    let linux = verify_layouts(instances.instances(), StorageTarget::LinuxX8664V1)
        .expect("actual semantic-demand Linux layouts");
    for layouts in [&linear, &linux] {
        assert_eq!(layouts.source_map_identity(), sources.identity());
        assert_eq!(
            layouts.types().map(zryna_layout::generic_v1::TypeView::key).collect::<Vec<_>>(),
            instances.instances().type_keys().collect::<Vec<_>>()
        );
    }
    assert_eq!(linear.universe_identity(), linux.universe_identity());
    let functions = instances.instances().function_keys().collect::<Vec<_>>();
    if case == "vec-copy" {
        // pack<i32>, authored from the frozen original declaration/call, not emitted claims.
        assert_eq!(functions, vec![b"\x40\0\0\0\0\0\0\0\0\x01\0\0\0\x01\0\0\0\x01".as_slice()]);
    } else {
        assert!(functions.is_empty(), "unused templates create no closed function instance");
    }
    let claim = execution_v1::produce_claim(&instances, &linear, &linux)
        .expect("genuine whole-domain untrusted claim");
    inspect(&sources, &syntax, &linear, &linux, &claim);
}
