//! Exact named packets and semantic-demand-owned layouts; no fabricated execution proof.
use sha2::{Digest, Sha256};
use std::path::Path;
use zryna_ir::generic_v1::execution_v1;
use zryna_layout::StorageTarget;
use zryna_ownership_runtime_abi::generic_v1 as runtime;
use zryna_semantics::bounded_generics_v1::{
    SemanticInput,
    body_types::check_execution_body_types as check_body_types,
    instantiation::{execution_v1 as semantic_execution, layouts::verify_layouts},
    resolve_declarations,
};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5::{VerifiedProjectSyntaxV5, decode_snapshot, verify_snapshot};
pub(super) const PROVIDERS: [&str; 2] = ["native", "typescript"];
fn part(case: &str) -> &'static str {
    if case == "called-indexed-shared-copy" {
        "caller"
    } else {
        assert!(matches!(case, "indexed-push-copy" | "indexed-push-rhs-relocates-owner"));
        "push"
    }
}
fn read(case: &str, name: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/m7-generic-execution-indexed-continuations")
            .join(part(case))
            .join(name),
    )
    .expect("frozen genuine dynamic indexed source input")
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn pinned(case: &str, name: &str) -> Vec<u8> {
    let identities: serde_json::Value = serde_json::from_str(include_str!("input-identities.json"))
        .expect("pre-test frozen identities");
    let name_key = format!("{}/{name}", part(case));
    let bytes = read(case, name);
    assert_eq!(
        u64::try_from(bytes.len()).expect("bounded input"),
        identities[&name_key]["bytes"].as_u64().expect("frozen count")
    );
    assert_eq!(digest(&bytes), identities[&name_key]["sha256"]);
    bytes
}
fn expected(case: &str) -> serde_json::Value {
    serde_json::from_slice(&pinned(case, "SOURCE_EXPECTED_FREEZE.json"))
        .expect("unchanged pre-authentication dynamic indexed source expectations")
}
pub(super) fn row(case: &str) -> serde_json::Value {
    expected(case)["cases"]
        .as_array()
        .expect("separate immutable additive source rows")
        .iter()
        .find(|row| row["case"] == case)
        .expect("frozen source case")
        .clone()
}
pub(super) fn project(case: &str, provider: &str) -> (SourceMap, VerifiedProjectSyntaxV5) {
    assert!(PROVIDERS.contains(&provider));
    let source = read(case, &format!("literals/{case}.zry"));
    let expected = row(case);
    assert_eq!(digest(&source), expected["sha256"]);
    let identities: serde_json::Value =
        serde_json::from_slice(&pinned(case, "packet-identities.json"))
            .expect("actual fresh process identities");
    let record = identities["records"]
        .as_array()
        .expect("actual named additive packets")
        .iter()
        .find(|row| row["case"] == case && row["provider"] == provider)
        .expect("honest provider identity");
    assert_eq!(record["exactSourceSha256"], expected["sha256"]);
    assert_eq!(record["acceptedSyntaxWithoutDiagnostics"], true);
    let packet = read(case, record["packet"].as_str().expect("actual packet path"));
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
pub(super) fn with_program(
    case: &str,
    provider: &str,
    check: impl FnOnce(&execution_v1::VerifiedExecution<'_>, &execution_v1::raw::Program, &[u8]),
) {
    let (sources, syntax) = project(case, provider);
    let entry = sources.verify_file_id(0).expect("entry");
    let declarations = resolve_declarations(
        SemanticInput::try_new(&syntax, &sources, entry).expect("original input"),
    )
    .expect("original declarations");
    let bodies = check_body_types(&declarations).expect("opaque original bodies");
    let instances = semantic_execution::discover(&bodies).expect("new original ownership");
    let linear = verify_layouts(instances.instances(), StorageTarget::Linear32V1).expect("linear");
    let linux = verify_layouts(instances.instances(), StorageTarget::LinuxX8664V1).expect("Linux");
    let runtime = runtime::verify_v1(
        runtime::raw_v1(&linear, &linux).expect("declarations"),
        &linear,
        &linux,
    )
    .expect("same-universe runtime");
    let claim = semantic_execution::produce_claim(&instances, &linear, &linux).expect("new claim");
    let wire = execution_v1::wire::encode(&claim).expect("distinct complete carrier");
    let program = execution_v1::verify(
        execution_v1::wire::decode(&wire).expect("execution carrier"),
        &syntax,
        &sources,
        entry,
        &linear,
        &linux,
        &runtime,
    )
    .expect("independent source/type/state/cleanup/ABI issuer");
    check(&program, &claim, &wire);
}

pub(super) fn diagnostic(case: &str) -> serde_json::Value {
    let oracle: serde_json::Value =
        serde_json::from_slice(&pinned(case, "DIAGNOSTIC_EXPECTED_FREEZE.json"))
            .expect("pre-provider exact diagnostic oracle");
    let expected = oracle["cases"]
        .as_array()
        .expect("one independent exact hostile")
        .iter()
        .find(|row| row["case"] == case)
        .expect("frozen hostile case")
        .clone();
    assert_eq!(row(case)["diagnostic"], expected);
    expected
}
pub(super) fn text(case: &str) -> String {
    let bytes = read(case, &format!("literals/{case}.zry"));
    assert_eq!(digest(&bytes), row(case)["sha256"]);
    String::from_utf8(bytes).expect("exact original UTF8")
}
