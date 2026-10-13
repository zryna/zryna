//! Own genuine C compiler/inspection prefixes and unavailable bindings refusal only.
use super::*;

#[test]
fn ordinary_rust_byte_c_sanitizer_compiler_prefix_refuses_unavailable_bindings_before_run() {
    let parent = material::evidence_parent().expect("external compiler-prefix evidence parent");
    let report = compiler_prefix::measure(&parent)
        .expect("fresh genuine Rust and all three C compiler/inspection prefixes");
    assert_eq!(report["policy"].as_str(), Some("None"));
    assert_eq!(report["targets_checked"].as_bool(), Some(false));
    assert_eq!(report["sanitizer_target_execution"].as_str(), Some("UNRUN"));
    assert_eq!(report["test_pass"].as_bool(), Some(false));
    assert_eq!(report["source_bytes"].as_u64(), Some(1696));
    assert_eq!(
        report["source_sha256"].as_str(),
        Some("26c4f32993bd1aabb118b8a3dfe66902bc623852564f562776218c366758826b")
    );
    let rust_records = report["rust_records"].as_array().expect("actual fresh Rust records");
    assert_eq!(rust_records.len(), 2);
    for (record, name) in rust_records.iter().zip(["rust-version", "rust-compile"]) {
        assert_eq!(record["name"].as_str(), Some(name));
        assert_eq!(record["success"].as_bool(), Some(true));
        assert!(record["diagnostic"].is_null());
        assert_eq!(record["error_raw_fields_available"].as_bool(), Some(true));
    }
    let cases = report["cases"].as_array().expect("three actual retained cases");
    assert_eq!(cases.len(), 3);
    let mut evidence_names = std::collections::BTreeSet::new();
    for (case, name) in cases.iter().zip([
        "sanitizer-positive",
        "sanitizer-heap-overrun",
        "sanitizer-signed-overflow",
    ]) {
        assert_eq!(case["case"].as_str(), Some(name));
        assert_eq!(case["phase"].as_str(), Some("unavailable-independent-sanitizer-bindings"));
        assert_eq!(case["expected_none_refusal"].as_str(), Some("ZRYNA-C4102"));
        assert_eq!(case["independently_reviewed_bindings"].as_bool(), Some(false));
        assert_eq!(case["targets_checked"].as_bool(), Some(false));
        assert_eq!(case["sanitizer_target_execution"].as_str(), Some("UNRUN"));
        assert_eq!(case["whole_sanitizer_test_pass"].as_bool(), Some(false));
        assert_eq!(case["cleanup_errors"].as_bool(), Some(false));
        assert_eq!(case["compiler_inspection_records_complete"].as_bool(), Some(true));
        assert!(evidence_names.insert(case["evidence"].as_str().expect("own case evidence")));
        let records = case["records"].as_array().expect("complete originating C records");
        assert_eq!(records.len(), 4, "no sanitizer target run record");
        for (record, expected) in
            records.iter().zip(["compile-client", "compile-runtime", "link", "inspect-shim"])
        {
            assert_eq!(record["name"].as_str(), Some(expected));
            assert_eq!(record["success"].as_bool(), Some(true));
            assert!(record["diagnostic"].is_null());
            assert_eq!(record["error_raw_fields_available"].as_bool(), Some(true));
        }
        assert!(!records.iter().any(|record| record["name"].as_str() == Some("run")));
    }
    prefix_capture::retain_if_requested(&parent, &report)
        .expect("closed opt-in compiler-prefix report retention");
}
