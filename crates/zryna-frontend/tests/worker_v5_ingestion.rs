//! Production TypeScript ingestion is compared to independent bytes and hostile transport claims.
use std::time::Duration;
use zryna_frontend::{WorkerFailure, WorkerLimitsV5};
use zryna_source::{SourceFileInput, SourceMap};

#[path = "worker_v5_ingestion/duplicate_arms.rs"]
mod duplicate_arms;
#[path = "worker_v5_ingestion/duplicate_struct_fields.rs"]
mod duplicate_struct_fields;
#[path = "worker_v5_ingestion/support.rs"]
mod support;
use support::*;

#[test]
fn pinned_worker_seals_all_independent_fixtures_without_changing_their_bytes() {
    for name in [
        "reference",
        "precedence",
        "control",
        "operations",
        "keyword-script",
        "keyword-module",
        "copy",
        "copy-cross",
        "m7-generic-owned-fixtures-local",
        "m7-generic-owned-fixtures-cross",
        "m7-generic-owned-cfg-local",
        "m7-generic-owned-cfg-cross",
    ] {
        let (sources, raw) = fixture(name);
        let first = production().analyze_verified_v5(&sources).unwrap_or_else(|error| {
            panic!("{name}: {error:?}; Node launch probe: {}", launch_diagnostic())
        });
        let second = production().analyze_verified_v5(&sources).expect("fresh process");
        assert!(first.is_bound_to(&sources));
        assert!(!first.is_bound_to(&fixture(name).0));
        assert_eq!(serde_json::to_value(first.files()).expect("wire"), raw["files"], "{name}");
        assert_eq!(
            serde_json::to_value(first.files()).expect("first"),
            serde_json::to_value(second.files()).expect("second")
        );
    }
}

#[test]
fn utf8_newline_and_nested_generic_ownership_boundaries_are_source_authenticated() {
    for separator in ["\n", "\r", "\u{2028}", "\u{2029}"] {
        let text = format!("// 😀{separator}function p(): i32 {{ return 7; }}");
        let sources = SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text }])
            .expect("source");
        let seal = production()
            .analyze_verified_v5(&sources)
            .expect("line comment ends before declaration");
        assert_eq!(
            seal.files()[0].functions[0].span.start as usize,
            "// 😀".len() + separator.len()
        );
        for gap in [separator.to_owned(), format!("/*{separator}*/")] {
            let text = format!("function p(): i32 {{ return{gap}7; }}");
            let sources = SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text }])
                .expect("source");
            let error = production()
                .analyze_verified_v5(&sources)
                .expect_err("ASI has no representable valued return");
            assert_eq!(error.failure(), WorkerFailure::ProviderRejected);
        }
    }
    let text = "function p(): i32 { const x: FixedArray<Result<i32, i32>, 1> = FixedArray<Result<i32, i32>, 1>([Result.ok<i32,i32>(7)]); return 7; }";
    let sources =
        SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: text.into() }])
            .expect("source");
    let seal = production()
        .analyze_verified_v5(&sources)
        .expect("nested comma and lexical type ownership");
    assert!(seal.is_bound_to(&sources));
    for text in [
        "function café(): i32 { return 7; }",
        "function p(x: FixedArray<i32, +1>): i32 { return 7; }",
        "function p(x: FixedArray<i32, 01>): i32 { return 7; }",
    ] {
        let sources =
            SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: text.into() }])
                .expect("source");
        assert_eq!(
            production().analyze_verified_v5(&sources).expect_err("unsupported spelling").failure(),
            WorkerFailure::ProviderRejected
        );
    }
}

#[test]
fn handshake_mismatch_never_writes_analysis_and_cannot_obtain_a_v5_seal() {
    for (mode, failure) in [
        ("identity", WorkerFailure::ProviderIdentity),
        ("version", WorkerFailure::ProviderVersion),
        ("protocol", WorkerFailure::ProviderProtocol),
        ("capability", WorkerFailure::ProviderCapabilities),
        ("extra-capability", WorkerFailure::InvalidResponse),
        ("missing-capability", WorkerFailure::InvalidResponse),
    ] {
        let seen =
            std::env::temp_dir().join(format!("zryna-v5-{}-{mode}.seen", std::process::id()));
        std::fs::write(&seen, "").expect("empty marker");
        let error = hostile(mode, WorkerLimitsV5::default(), Some(&seen))
            .analyze_verified_v5(&fixture("reference").0)
            .expect_err(mode);
        assert_eq!(error.failure(), failure, "{mode}");
        assert_eq!(std::fs::read_to_string(&seen).expect("marker"), "handshake\n", "{mode}");
        std::fs::remove_file(seen).expect("marker cleanup");
    }
}

#[test]
fn independent_wire_mutations_fail_before_source_authority() {
    for mode in [
        "unknown",
        "missing-nullable",
        "wrong-schema",
        "wrong-id",
        "duplicate",
        "nested-duplicate",
        "trailing-value",
    ] {
        let error = hostile(mode, WorkerLimitsV5::default(), None)
            .analyze_verified_v5(&fixture("reference").0)
            .expect_err(mode);
        assert_eq!(error.failure(), WorkerFailure::InvalidResponse, "{mode}");
        assert!(error.diagnostics().is_empty(), "no invented semantic diagnostic");
    }
}

#[test]
fn source_producer_error_codes_remain_untrusted_transport_rejections() {
    for mode in ["provider-declaration-reject", "provider-argument-reject"] {
        let error = hostile(mode, WorkerLimitsV5::default(), None)
            .analyze_verified_v5(&fixture("reference").0)
            .expect_err("a provider error cannot authenticate source semantics");
        assert_eq!(error.failure(), WorkerFailure::ProviderRejected);
        assert!(error.diagnostics().is_empty(), "provider-selected D/M codes are advisory");
    }
}

#[test]
fn source_forgery_and_changed_source_fail_at_the_authenticated_boundary() {
    let (sources, _) = fixture("reference");
    let error = hostile("span", WorkerLimitsV5::default(), None)
        .analyze_verified_v5(&sources)
        .expect_err("forged span");
    assert_eq!(error.failure(), WorkerFailure::SnapshotVerification);
    assert!(error.diagnostics().iter().all(|diagnostic| diagnostic.code == "ZRYNA-Y5001"));
    let folder = root().join("tests/m7-syntax-fixtures");
    let changed = SourceMap::build(vec![
        SourceFileInput {
            path: "main.zry".into(),
            text: std::fs::read_to_string(folder.join("main.zry"))
                .expect("source")
                .replace("score", "scare"),
        },
        SourceFileInput {
            path: "values.zry".into(),
            text: std::fs::read_to_string(folder.join("values.zry")).expect("source"),
        },
    ])
    .expect("changed map");
    let error = hostile("valid", WorkerLimitsV5::default(), None)
        .analyze_verified_v5(&changed)
        .expect_err("foreign bytes");
    assert_eq!(error.failure(), WorkerFailure::SnapshotVerification);
}

#[test]
fn invalid_bound_and_strict_keyword_roles_are_checked_by_source_verification() {
    for (text, code) in [
        ("function id<T extends OtherValue>(value: T): T { return value; }", "ZRYNA-D7001"),
        (
            include_str!("../../../tests/m7-syntax-fixtures/keyword-strict-target.zry"),
            "ZRYNA-Y5001",
        ),
    ] {
        let sources =
            SourceMap::build(vec![SourceFileInput { path: "main.zry".into(), text: text.into() }])
                .expect("source");
        let error = production().analyze_verified_v5(&sources).expect_err("no seal");
        assert_eq!(error.failure(), WorkerFailure::SnapshotVerification);
        assert!(error.diagnostics().iter().any(|diagnostic| diagnostic.code == code), "{error:?}");
    }
}

#[test]
fn provider_framing_exit_and_byte_caps_reject_without_partial_seal() {
    for (mode, expected) in [
        ("provider-reject", WorkerFailure::ProviderRejected),
        ("invalid-utf8", WorkerFailure::InvalidResponse),
        ("extra-response", WorkerFailure::InvalidResponse),
        ("exit", WorkerFailure::ProcessExit),
    ] {
        let error = hostile(mode, WorkerLimitsV5::default(), None)
            .analyze_verified_v5(&fixture("reference").0)
            .expect_err(mode);
        assert_eq!(error.failure(), expected, "{mode}");
    }
    let limits = WorkerLimitsV5::new(Duration::from_secs(5), 512, 4096).expect("stdout cap");
    let error = hostile("valid", limits, None)
        .analyze_verified_v5(&fixture("reference").0)
        .expect_err("overflow");
    assert_eq!(error.failure(), WorkerFailure::OutputLimit);
    let limits = WorkerLimitsV5::new(Duration::from_secs(5), 64 * 1024, 512).expect("stderr cap");
    let error = hostile("stderr", limits, None)
        .analyze_verified_v5(&fixture("reference").0)
        .expect_err("overflow");
    assert_eq!(error.failure(), WorkerFailure::OutputLimit);
}

#[test]
fn v5_limits_and_timeout_are_tightening_only() {
    assert!(WorkerLimitsV5::new(Duration::ZERO, 1, 1).is_err());
    assert!(WorkerLimitsV5::new(Duration::from_secs(31), 1, 1).is_err());
    assert!(WorkerLimitsV5::new(Duration::from_secs(1), usize::MAX, 1).is_err());
    let frontend = hostile("timeout", WorkerLimitsV5::default(), None);
    let error = frontend
        .analyze_verified_v5_with_timeout(&fixture("reference").0, Duration::from_secs(1))
        .expect_err("timeout");
    assert_eq!(error.failure(), WorkerFailure::Timeout);
}
