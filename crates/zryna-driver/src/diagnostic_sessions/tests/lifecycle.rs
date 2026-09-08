use std::time::Duration;

use serde_json::json;

use super::{ready_session, request, sources};
use crate::diagnostic_sessions::{
    DiagnosticSession, DiagnosticSessionError, MAX_REVISION, QUERY_DEADLINE, QueryReason,
    QueryStatus,
};

#[test]
fn same_length_replacement_rejects_old_work_and_replay() {
    let (mut session, first, now) = ready_session("let x = 1;\n");
    let bytes = request("old", first, 100_000, "diagnostics", json!({}));
    let pending = session
        .begin_diagnostics(&bytes, now)
        .unwrap_or_else(|response| panic!("first request must begin: {response:?}"));

    let second = session
        .admit_diagnostics(sources("src/main.zry", "let y = 1;\n"), &[])
        .unwrap_or_else(|error| panic!("replacement must be admitted: {error}"));
    assert_ne!(first.handle(), second.handle());
    assert_eq!(session.finish_diagnostics(pending, now).status(), QueryStatus::Stale);
    let replay = session
        .begin_diagnostics(&bytes, now)
        .expect_err("old same-length source must remain stale");
    assert_eq!(replay.reason(), Some(QueryReason::Snapshot));
}

#[test]
fn replacement_is_stale_after_internal_readiness_and_work() {
    let mut session = DiagnosticSession::try_new()
        .unwrap_or_else(|error| panic!("session identity must be available: {error}"));
    let unready = session
        .admit_unready(sources("src/main.zry", "let x = 1;\n"))
        .unwrap_or_else(|error| panic!("source-only revision must fit: {error}"));
    let now = std::time::Instant::now();
    let old_unready = session
        .begin_diagnostics(&request("unready", unready, 100_000, "diagnostics", json!({})), now)
        .unwrap_or_else(|response| panic!("unready query must begin: {response:?}"));
    let ready = session
        .admit_diagnostics(sources("src/main.zry", "let y = 1;\n"), &[])
        .unwrap_or_else(|error| panic!("replacement must fit: {error}"));
    assert_eq!(session.finish_diagnostics(old_unready, now).status(), QueryStatus::Stale);

    let old_low_work = session
        .begin_diagnostics(&request("low-work", ready, 1, "diagnostics", json!({})), now)
        .unwrap_or_else(|response| panic!("low-work query must begin: {response:?}"));
    session
        .admit_diagnostics(sources("src/main.zry", "let z = 1;\n"), &[])
        .unwrap_or_else(|error| panic!("second replacement must fit: {error}"));
    let response = session.finish_diagnostics(old_low_work, now);
    assert_eq!(
        (response.status(), response.reason()),
        (QueryStatus::Stale, Some(QueryReason::Snapshot))
    );
}

#[test]
fn foreign_session_handle_is_never_substituted() {
    let (first_session, first, now) = ready_session("let x = 1;\n");
    let (mut second_session, second, _) = ready_session("let x = 1;\n");
    assert_ne!(first.handle(), second.handle());
    assert_eq!(first.source_fingerprint(), second.source_fingerprint());

    let bytes = request("foreign", first, 100_000, "diagnostics", json!({}));
    let response =
        second_session.begin_diagnostics(&bytes, now).expect_err("foreign handle must be stale");
    assert_eq!(
        (response.status(), response.reason()),
        (QueryStatus::Stale, Some(QueryReason::Snapshot))
    );
    drop(first_session);
}

#[test]
fn oldest_revision_is_evicted_without_handle_reuse() {
    let (mut session, first, now) = ready_session("let x = 1;\n");
    let second = session
        .admit_diagnostics(sources("src/main.zry", "let y = 1;\n"), &[])
        .unwrap_or_else(|error| panic!("second revision must fit: {error}"));
    let third = session
        .admit_diagnostics(sources("src/main.zry", "let z = 1;\n"), &[])
        .unwrap_or_else(|error| panic!("third revision must fit after eviction: {error}"));
    assert_eq!(session.retained_revisions(), 2);
    assert_ne!(first.handle(), second.handle());
    assert_ne!(second.handle(), third.handle());

    let response = session
        .begin_diagnostics(&request("evicted", first, 100_000, "diagnostics", json!({})), now)
        .expect_err("evicted revision must be stale");
    assert_eq!(response.reason(), Some(QueryReason::Snapshot));
}

#[test]
fn revision_exhaustion_requires_a_new_session() {
    let mut session = DiagnosticSession::try_new()
        .unwrap_or_else(|error| panic!("session identity must be available: {error}"));
    session.next_revision = MAX_REVISION;
    let last = session
        .admit_diagnostics(sources("src/main.zry", ""), &[])
        .unwrap_or_else(|error| panic!("maximum revision is inclusive: {error}"));
    assert_eq!(last.revision(), MAX_REVISION);
    assert!(matches!(
        session.admit_diagnostics(sources("src/main.zry", "x"), &[]),
        Err(DiagnosticSessionError::RevisionExhausted)
    ));
    assert_eq!(session.active_revision(), Some(last));
}

#[test]
fn cancellation_and_deadline_release_slots_and_recover() {
    let (mut session, revision, now) = ready_session("let x = 1;\n");
    let bytes = request("repeat", revision, 100_000, "diagnostics", json!({}));
    let cancelled = session
        .begin_diagnostics(&bytes, now)
        .unwrap_or_else(|response| panic!("request must begin: {response:?}"));
    assert!(session.cancel("repeat"));
    let response = session.finish_diagnostics(cancelled, now);
    assert_eq!(
        (response.status(), response.reason()),
        (QueryStatus::Cancelled, Some(QueryReason::Request))
    );

    let expired = session
        .begin_diagnostics(&bytes, now)
        .unwrap_or_else(|response| panic!("cancelled ID must be reusable: {response:?}"));
    let response = session.finish_diagnostics(expired, now + QUERY_DEADLINE);
    assert_eq!(
        (response.status(), response.reason()),
        (QueryStatus::Cancelled, Some(QueryReason::Deadline))
    );

    let recovered = session
        .begin_diagnostics(&bytes, now + Duration::from_secs(31))
        .unwrap_or_else(|response| panic!("deadline cleanup must recover: {response:?}"));
    assert_eq!(
        session.finish_diagnostics(recovered, now + Duration::from_secs(31)).status(),
        QueryStatus::Ok
    );
}

#[test]
fn cancelled_completion_cannot_remove_a_reused_request_id() {
    let (mut session, revision, now) = ready_session("let x = 1;\n");
    let bytes = request("reuse", revision, 100_000, "diagnostics", json!({}));
    let cancelled = session
        .begin_diagnostics(&bytes, now)
        .unwrap_or_else(|response| panic!("first request must begin: {response:?}"));
    assert!(session.cancel("reuse"));
    let replacement = session
        .begin_diagnostics(&bytes, now)
        .unwrap_or_else(|response| panic!("replacement request must begin: {response:?}"));
    assert_eq!(session.finish_diagnostics(cancelled, now).status(), QueryStatus::Cancelled);
    assert_eq!(session.finish_diagnostics(replacement, now).status(), QueryStatus::Ok);
}

#[test]
fn unready_revision_never_becomes_empty_success() {
    let mut session = DiagnosticSession::try_new()
        .unwrap_or_else(|error| panic!("session identity must be available: {error}"));
    let revision = session
        .admit_unready(sources("src/main.zry", "let x = 1;\n"))
        .unwrap_or_else(|error| panic!("source-only revision must be admitted: {error}"));
    let now = std::time::Instant::now();
    let pending = session
        .begin_diagnostics(&request("pending", revision, 100_000, "diagnostics", json!({})), now)
        .unwrap_or_else(|response| panic!("request must begin: {response:?}"));
    let response = session.finish_diagnostics(pending, now);
    assert_eq!(
        (response.status(), response.reason()),
        (QueryStatus::Unavailable, Some(QueryReason::Analysis))
    );
}
