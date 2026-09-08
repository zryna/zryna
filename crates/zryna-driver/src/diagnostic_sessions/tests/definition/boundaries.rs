use std::time::Duration;

use super::{definition_request, semantic_session};
use crate::diagnostic_sessions::{
    MAX_IN_FLIGHT_REQUESTS, QUERY_DEADLINE, QueryReason, QueryStatus,
};

#[test]
fn cancelled_definition_completion_cannot_release_a_reused_id() {
    let (mut session, revision, now) = semantic_session();
    let bytes = definition_request("reuse", revision, 100, "src/main.zry", 47);
    let old = session.begin_definition(&bytes, now).expect("first definition must begin");
    assert!(session.cancel("reuse"));
    let current = session.begin_definition(&bytes, now).expect("cancelled ID must be reusable");
    let response = session.finish_definition(old, now);
    assert_eq!(
        (response.status(), response.reason()),
        (QueryStatus::Cancelled, Some(QueryReason::Request))
    );
    let duplicate = session.begin_definition(&bytes, now).expect_err("current ID remains reserved");
    assert_eq!(
        (duplicate.status(), duplicate.reason()),
        (QueryStatus::Malformed, Some(QueryReason::Shape))
    );
    assert!(session.cancel("reuse"));
    assert_eq!(session.finish_definition(current, now).status(), QueryStatus::Cancelled);
    let recovered = session.begin_definition(&bytes, now).expect("completed ID must recover");
    assert_eq!(session.finish_definition(recovered, now).status(), QueryStatus::Ok);
}

#[test]
fn definition_deadline_is_exact_and_expired_completion_preserves_reuse() {
    let (mut session, revision, now) = semantic_session();
    let bytes = definition_request("deadline", revision, 100, "src/main.zry", 47);
    let before = session.begin_definition(&bytes, now).expect("definition must begin");
    assert_eq!(
        session
            .finish_definition(
                before,
                (now + QUERY_DEADLINE)
                    .checked_sub(Duration::from_nanos(1))
                    .expect("before deadline")
            )
            .status(),
        QueryStatus::Ok
    );
    let old = session.begin_definition(&bytes, now).expect("deadline fixture must begin");
    let later = now + QUERY_DEADLINE;
    let current = session.begin_definition(&bytes, later).expect("deadline must release old slot");
    let response = session.finish_definition(old, later);
    assert_eq!(
        (response.status(), response.reason()),
        (QueryStatus::Cancelled, Some(QueryReason::Deadline))
    );
    assert_eq!(
        session.begin_definition(&bytes, later).expect_err("new slot stays reserved").reason(),
        Some(QueryReason::Shape)
    );
    assert_eq!(session.finish_definition(current, later).status(), QueryStatus::Ok);
    let exact = session.begin_definition(&bytes, later).expect("completed slot must recover");
    let response = session.finish_definition(exact, later + QUERY_DEADLINE);
    assert_eq!(
        (response.status(), response.reason()),
        (QueryStatus::Cancelled, Some(QueryReason::Deadline))
    );
}

#[test]
fn definition_queue_recovers_from_cancellation_and_deadline() {
    let (mut session, revision, now) = semantic_session();
    let mut pending = Vec::new();
    for index in 0..MAX_IN_FLIGHT_REQUESTS {
        let bytes = definition_request(&format!("q{index}"), revision, 100, "src/main.zry", 47);
        pending.push(session.begin_definition(&bytes, now).expect("each queue slot must fit"));
    }
    let bytes = definition_request("extra", revision, 100, "src/main.zry", 47);
    let response = session.begin_definition(&bytes, now).expect_err("full queue must reject");
    assert_eq!(
        (response.status(), response.reason()),
        (QueryStatus::OverBudget, Some(QueryReason::Session))
    );
    assert!(session.cancel("q0"));
    let current = session.begin_definition(&bytes, now).expect("cancelled slot must be reusable");
    let old = pending.remove(0);
    assert_eq!(session.finish_definition(old, now).status(), QueryStatus::Cancelled);
    let overflow = definition_request("overflow", revision, 100, "src/main.zry", 47);
    assert_eq!(
        session.begin_definition(&overflow, now).expect_err("queue remains full").reason(),
        Some(QueryReason::Session)
    );
    let later = now + QUERY_DEADLINE;
    let recovered = session.begin_definition(&overflow, later).expect("expired queue must recover");
    assert_eq!(session.finish_definition(recovered, later).status(), QueryStatus::Ok);
    for expired in pending.into_iter().chain(std::iter::once(current)) {
        let response = session.finish_definition(expired, later);
        assert_eq!(
            (response.status(), response.reason()),
            (QueryStatus::Cancelled, Some(QueryReason::Deadline))
        );
    }
}

#[test]
fn foreign_definition_completion_cannot_release_local_request() {
    let (mut first, first_revision, now) = semantic_session();
    let (mut second, second_revision, _) = semantic_session();
    let foreign = first
        .begin_definition(
            &definition_request("same-id", first_revision, 100, "src/main.zry", 47),
            now,
        )
        .expect("foreign definition must begin");
    let bytes = definition_request("same-id", second_revision, 100, "src/main.zry", 47);
    let local = second.begin_definition(&bytes, now).expect("local definition must begin");
    let response = second.finish_definition(foreign, now);
    assert_eq!(
        (response.status(), response.reason()),
        (QueryStatus::Stale, Some(QueryReason::Snapshot))
    );
    assert_eq!(
        second.begin_definition(&bytes, now).expect_err("local slot stays reserved").reason(),
        Some(QueryReason::Shape)
    );
    assert_eq!(second.finish_definition(local, now).status(), QueryStatus::Ok);
    assert!(first.cancel("same-id"));
}

#[test]
fn stale_definition_completion_cannot_release_new_revision_request() {
    let (mut session, first, now) = semantic_session();
    let old = session
        .begin_definition(&definition_request("revision", first, 100, "src/main.zry", 47), now)
        .expect("old definition must begin");
    let map = super::sources("src/main.zry", super::SOURCE);
    let syntax = super::verify_snapshot(super::raw_snapshot(), &map)
        .expect("replacement syntax must authenticate");
    let second = session.admit_semantics(map, &syntax).expect("replacement revision must fit");
    assert_ne!(first.handle(), second.handle());
    let bytes = definition_request("revision", second, 100, "src/main.zry", 47);
    let current = session.begin_definition(&bytes, now).expect("new revision ID must be reusable");
    let response = session.finish_definition(old, now);
    assert_eq!(
        (response.status(), response.reason()),
        (QueryStatus::Stale, Some(QueryReason::Snapshot))
    );
    assert_eq!(
        session
            .begin_definition(&bytes, now)
            .expect_err("new revision slot stays reserved")
            .reason(),
        Some(QueryReason::Shape)
    );
    assert_eq!(session.finish_definition(current, now).status(), QueryStatus::Ok);
}
