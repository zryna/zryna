use serde_json::json;
use zryna_diagnostics::{Diagnostic, protocol_v2::ProtocolError};

use super::{ready_session, request, sources};
use crate::diagnostic_sessions::{DiagnosticSessionError, QueryReason, QueryStatus};

#[test]
fn foreign_same_length_span_is_rejected_before_retention() {
    let original = sources("src/main.zry", "let x = 1;\n");
    let file =
        original.verify_file_id(0).unwrap_or_else(|error| panic!("file zero must exist: {error}"));
    let span =
        original.span(file, 4, 5).unwrap_or_else(|error| panic!("x span must be valid: {error}"));
    let diagnostic = Diagnostic::error_at("ZRYNA-T1001", span, "inert", "inert");
    let mut session = crate::diagnostic_sessions::DiagnosticSession::try_new()
        .unwrap_or_else(|error| panic!("session must exist: {error}"));
    let foreign = sources("src/main.zry", "let y = 1;\n");
    assert!(matches!(
        session.admit_diagnostics(foreign, &[diagnostic]),
        Err(DiagnosticSessionError::Diagnostics(ProtocolError::Source))
    ));
    assert_eq!(session.active_revision(), None);
}

#[test]
fn malformed_shape_precedes_stale_lookup() {
    let (mut session, revision, now) = ready_session("let x = 1;\n");
    let stale_revision = revision.revision() + 1;
    let duplicate = format!(
        "{{\"query_version\":1,\"request_id\":\"q\",\"snapshot\":\"{}\",\"revision\":{stale_revision},\"revision\":{stale_revision},\"method\":\"diagnostics\",\"params\":{{}},\"limits\":{{\"work\":100000,\"results\":10000}}}}",
        revision.handle()
    );
    let response = session
        .begin_diagnostics(duplicate.as_bytes(), now)
        .expect_err("duplicate field must reject");
    assert_eq!(
        (response.status(), response.reason()),
        (QueryStatus::Malformed, Some(QueryReason::Shape))
    );
    assert_eq!(response.encoded(), None);
}

#[test]
fn version_and_method_follow_snapshot_precedence() {
    let (mut session, revision, now) = ready_session("let x = 1;\n");
    let mut version: serde_json::Value =
        serde_json::from_slice(&request("version", revision, 100_000, "diagnostics", json!({})))
            .unwrap_or_else(|error| panic!("request must decode: {error}"));
    version["query_version"] = json!(2);
    let response = session
        .begin_diagnostics(
            &serde_json::to_vec(&version)
                .unwrap_or_else(|error| panic!("request must encode: {error}")),
            now,
        )
        .expect_err("version two must reject");
    assert_eq!(
        (response.status(), response.reason()),
        (QueryStatus::Unsupported, Some(QueryReason::Version))
    );

    let stale_method = request(
        "method",
        crate::diagnostic_sessions::DiagnosticRevision {
            handle: revision.handle(),
            revision: revision.revision() + 1,
            source_fingerprint: revision.source_fingerprint(),
        },
        100_000,
        "hover",
        json!({"path":"src/main.zry","byte_offset":0}),
    );
    let response = session
        .begin_diagnostics(&stale_method, now)
        .expect_err("stale lookup must precede method lookup");
    assert_eq!(response.reason(), Some(QueryReason::Snapshot));

    let response = session
        .begin_diagnostics(
            &request("method", revision, 100_000, "hover", json!({"path":"src/main.zry"})),
            now,
        )
        .expect_err("semantic method must stay unsupported");
    assert_eq!(response.reason(), Some(QueryReason::Method));
}

#[test]
fn duplicate_in_flight_id_rejects_and_does_not_remove_original() {
    let (mut session, revision, now) = ready_session("let x = 1;\n");
    let bytes = request("same", revision, 100_000, "diagnostics", json!({}));
    let first = session
        .begin_diagnostics(&bytes, now)
        .unwrap_or_else(|response| panic!("first request must begin: {response:?}"));
    let duplicate = session.begin_diagnostics(&bytes, now).expect_err("duplicate ID must reject");
    assert_eq!(
        (duplicate.status(), duplicate.reason()),
        (QueryStatus::Malformed, Some(QueryReason::Shape))
    );
    assert_eq!(session.finish_diagnostics(first, now).status(), QueryStatus::Ok);
}

#[test]
fn unicode_crlf_eof_ranges_remain_source_bound() {
    let text = "// é😀e\u{301}\r\nlet x = 1;\n";
    let map = sources("src/unicode.zry", text);
    let file =
        map.verify_file_id(0).unwrap_or_else(|error| panic!("file zero must exist: {error}"));
    let eof = u32::try_from(text.len()).unwrap_or_else(|_| panic!("test text must fit u32"));
    let span =
        map.span(file, eof, eof).unwrap_or_else(|error| panic!("EOF span must be valid: {error}"));
    let diagnostic = Diagnostic::warning_at("ZRYNA-T1002", span, "inert", "inert");
    let mut session = crate::diagnostic_sessions::DiagnosticSession::try_new()
        .unwrap_or_else(|error| panic!("session must exist: {error}"));
    let revision = session
        .admit_diagnostics(map, &[diagnostic])
        .unwrap_or_else(|error| panic!("Unicode report must be retained: {error}"));
    let now = std::time::Instant::now();
    let pending = session
        .begin_diagnostics(&request("unicode", revision, 100_000, "diagnostics", json!({})), now)
        .unwrap_or_else(|response| panic!("query must begin: {response:?}"));
    let response = session.finish_diagnostics(pending, now);
    assert_eq!(response.status(), QueryStatus::Ok);
    let encoded = response.encoded().unwrap_or_else(|| panic!("success must be encoded"));
    assert!(encoded.contains(&format!("\"byte_start\":{eof},\"byte_end\":{eof}")));
}
