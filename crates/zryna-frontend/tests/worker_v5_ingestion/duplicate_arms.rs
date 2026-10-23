//! Duplicate keys are original syntax facts; semantic rejection has a separate authority.

use serde_json::Value;
use zryna_frontend::{WorkerFailure, native_lexer, native_parser::v5::parse_v5_candidate};
use zryna_source::{SourceFileInput, SourceMap};
use zryna_syntax::v5::{RawProjectSyntaxSnapshot, verify_snapshot};

use super::production;

const SOURCE: &str =
    include_str!("../../../../tests/m7-generic-catalogue-fixtures/duplicate-arm/main.zry");
const REFERENCE: &[u8] = include_bytes!(
    "../../../../tests/m7-generic-catalogue-fixtures/duplicate-arm/independent.json"
);

fn sources(text: &str) -> SourceMap {
    SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: text.into() }])
        .expect("original immutable source")
}

fn reference() -> Value {
    serde_json::from_slice(REFERENCE).expect("independent complete source/arena fixture")
}

fn assert_all_original_arms(files: &Value) {
    let expected = reference();
    assert_eq!(files, &expected["files"], "every original source token/arena claim is retained");
    let arms = files[0]["functions"][0]["body"]["expressions"][4]["kind"]["arms"]
        .as_array()
        .expect("original match arms");
    assert_eq!(arms.len(), 3, "a producer must not discard the duplicate arm");
    assert_eq!(arms[0]["variant"]["text"], "none");
    assert_eq!(arms[1]["variant"]["text"], "none");
    assert_eq!(arms[2]["variant"]["text"], "some");
    assert_eq!(arms[1]["span"]["start"], 81);
    assert_eq!(&SOURCE.as_bytes()[81..94], b"\"Option.none\"");
}

#[test]
fn native_v5_preserves_duplicate_arm_raw_candidate_for_source_authentication() {
    let map = sources(SOURCE);
    let lexed = native_lexer::lex(&map).expect("original lexical facts");
    let raw = parse_v5_candidate(&map, &lexed)
        .expect("duplicate semantic keys must remain source-faithful untrusted syntax");
    assert!(raw.diagnostics.is_empty());
    assert_all_original_arms(&serde_json::to_value(&raw.files).expect("raw arena facts"));
    let verified = verify_snapshot(raw, &map).expect("complete original source/arena proof");
    assert!(verified.is_bound_to(&map));
    // No semantic, IR, layout, ABI, backend, or executable authority is asserted here.
}

#[test]
fn typescript_v5_preserves_duplicate_arm_inventory_before_semantic_rejection() {
    let map = sources(SOURCE);
    let verified = production()
        .analyze_verified_v5(&map)
        .expect("actual TypeScript worker must return a complete raw DTO for syntax verification");
    assert!(verified.is_bound_to(&map));
    assert!(verified.advisories().is_empty());
    assert_all_original_arms(
        &serde_json::to_value(verified.files()).expect("verified source facts"),
    );
}

fn malformed_sources() -> [String; 3] {
    [
        SOURCE.trim_end().strip_suffix('}').expect("complete original closing brace").into(),
        SOURCE.replace("\"Option.none\": () => 1", "\"Option.none\": () => "),
        SOURCE.replace("\"Option.some\": (item) => item", "\"Option.some\": (item) => "),
    ]
}

#[test]
fn native_v5_duplicate_keys_do_not_admit_incomplete_or_malformed_source() {
    for source in malformed_sources() {
        let map = sources(&source);
        let lexed = native_lexer::lex(&map).expect("controls are lexically complete ASCII");
        let error = parse_v5_candidate(&map, &lexed).expect_err("no partial raw candidate");
        assert_eq!(error.diagnostic().code(), "ZRYNA-F2002");
    }
}

#[test]
fn typescript_v5_duplicate_keys_do_not_admit_incomplete_or_malformed_source() {
    for source in malformed_sources() {
        let error = production()
            .analyze_verified_v5(&sources(&source))
            .expect_err("actual producer must reject malformed source without a seal");
        assert_eq!(error.failure(), WorkerFailure::ProviderRejected);
        assert!(error.diagnostics().is_empty(), "provider errors remain untrusted");
    }
}

#[test]
fn omitted_duplicate_arm_cannot_authenticate_against_complete_source() {
    let mut raw = reference();
    raw["files"][0]["functions"][0]["body"]["expressions"][4]["kind"]["arms"]
        .as_array_mut()
        .expect("original arms")
        .remove(1);
    let raw: RawProjectSyntaxSnapshot = serde_json::from_value(raw).expect("closed hostile DTO");
    let errors = verify_snapshot(raw, &sources(SOURCE)).expect_err("omitted original source fails");
    assert!(errors.iter().all(|error| error.code == "ZRYNA-Y5001"));
}
