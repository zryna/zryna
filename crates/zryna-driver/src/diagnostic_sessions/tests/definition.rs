use std::time::Instant;

use serde_json::{Value, json};
use zryna_frontend::syntax_v2::{
    RawExpressionKind, RawExpressionSyntax, RawFunctionBodySyntax, RawFunctionSyntax,
    RawIdentifierSyntax, RawParameterSyntax, RawProjectSyntaxSnapshot, RawSourceUnit,
    RawStatementKind, RawStatementSyntax, RawTypeSyntax, RawTypeSyntaxKind, verify_snapshot,
};
use zryna_source::UntrustedSpan;

use super::{request, sources};
use crate::diagnostic_sessions::{DiagnosticSession, QueryReason, QueryStatus};

const SOURCE: &str = "export function identity(x: i32): i32 { return x; }\n";

fn span(start: u32, end: u32) -> UntrustedSpan {
    UntrustedSpan { file: 0, start, end }
}

fn raw_snapshot() -> RawProjectSyntaxSnapshot {
    RawProjectSyntaxSnapshot {
        schema_version: 2,
        files: vec![RawSourceUnit {
            id: 0,
            path: "src/main.zry".to_owned(),
            functions: vec![RawFunctionSyntax {
                span: span(0, 51),
                export_span: span(0, 6),
                function_span: span(7, 15),
                name: RawIdentifierSyntax { text: "identity".to_owned(), span: span(16, 24) },
                parameters: vec![RawParameterSyntax {
                    span: span(25, 31),
                    name: RawIdentifierSyntax { text: "x".to_owned(), span: span(25, 26) },
                    type_syntax: RawTypeSyntax {
                        span: span(28, 31),
                        kind: RawTypeSyntaxKind::Named { name: "i32".to_owned() },
                    },
                }],
                result_type: RawTypeSyntax {
                    span: span(34, 37),
                    kind: RawTypeSyntaxKind::Named { name: "i32".to_owned() },
                },
                body: RawFunctionBodySyntax {
                    span: span(38, 51),
                    statements: vec![RawStatementSyntax {
                        span: span(40, 49),
                        kind: RawStatementKind::Return { keyword_span: span(40, 46), value: 0 },
                    }],
                    expressions: vec![RawExpressionSyntax {
                        span: span(47, 48),
                        kind: RawExpressionKind::Reference {
                            name: RawIdentifierSyntax { text: "x".to_owned(), span: span(47, 48) },
                        },
                    }],
                },
            }],
        }],
        diagnostics: Vec::new(),
    }
}

fn semantic_session() -> (DiagnosticSession, super::DiagnosticRevision, Instant) {
    let map = sources("src/main.zry", SOURCE);
    let syntax = verify_snapshot(raw_snapshot(), &map)
        .unwrap_or_else(|errors| panic!("definition fixture syntax must authenticate: {errors:?}"));
    let mut session = DiagnosticSession::try_new()
        .unwrap_or_else(|error| panic!("session identity must be available: {error}"));
    let revision = session
        .admit_semantics(map, &syntax)
        .unwrap_or_else(|error| panic!("semantic revision must be admitted: {error}"));
    (session, revision, Instant::now())
}

fn definition_request(
    id: &str,
    revision: super::DiagnosticRevision,
    work: u64,
    path: &str,
    byte_offset: u32,
) -> Vec<u8> {
    let mut value: Value = serde_json::from_slice(&request(
        id,
        revision,
        work,
        "definition",
        json!({"path": path, "byte_offset": byte_offset}),
    ))
    .unwrap_or_else(|error| panic!("request must decode: {error}"));
    value["limits"]["results"] = json!(1);
    serde_json::to_vec(&value).unwrap_or_else(|error| panic!("request must encode: {error}"))
}

#[test]
fn parameter_use_resolves_to_exact_declaration_and_replays() {
    let (mut session, revision, now) = semantic_session();
    let bytes = definition_request("definition", revision, 4, "src/main.zry", 47);
    let first = session
        .begin_definition(&bytes, now)
        .unwrap_or_else(|response| panic!("definition must begin: {response:?}"));
    let first = session.finish_definition(first, now);
    assert_eq!(first.status(), QueryStatus::Ok);
    let expected = format!(
        "{{\"query_version\":1,\"request_id\":\"definition\",\"snapshot\":\"{}\",\"revision\":{},\"status\":\"ok\",\"result\":{{\"locations\":[{{\"path\":\"src/main.zry\",\"byte_start\":25,\"byte_end\":26}}]}}}}",
        revision.handle(),
        revision.revision()
    );
    assert_eq!(first.encoded(), Some(expected.as_str()));

    let replay = session
        .begin_definition(&bytes, now)
        .unwrap_or_else(|response| panic!("definition replay must begin: {response:?}"));
    assert_eq!(session.finish_definition(replay, now).encoded(), first.encoded());
}

#[test]
fn semantic_records_have_an_exact_cache_charge() {
    let (session, _, _) = semantic_session();
    let record =
        session.retained.back().unwrap_or_else(|| panic!("semantic revision must remain retained"));
    let definitions = record
        .definitions
        .as_deref()
        .unwrap_or_else(|| panic!("semantic revision must retain definition facts"));
    assert_eq!(definitions.cache_bytes(), Some(3 * 24));
    let report_bytes = record.report.as_deref().map_or(0, str::len);
    assert_eq!(session.cache_bytes(), "src/main.zry".len() + SOURCE.len() + report_bytes + 3 * 24);
}

#[test]
fn token_end_is_absent_and_low_work_recovers() {
    let (mut session, revision, now) = semantic_session();
    let absent = session
        .begin_definition(&definition_request("absent", revision, 4, "src/main.zry", 48), now)
        .unwrap_or_else(|response| panic!("absent lookup must begin: {response:?}"));
    let absent = session.finish_definition(absent, now);
    assert_eq!(
        (absent.status(), absent.reason()),
        (QueryStatus::Absent, Some(QueryReason::Symbol))
    );

    let low = session
        .begin_definition(&definition_request("low", revision, 3, "src/main.zry", 47), now)
        .unwrap_or_else(|response| panic!("low-work lookup must begin: {response:?}"));
    let low = session.finish_definition(low, now);
    assert_eq!((low.status(), low.reason()), (QueryStatus::OverBudget, Some(QueryReason::Work)));
    let recovered = session
        .begin_definition(&definition_request("recovered", revision, 4, "src/main.zry", 47), now)
        .unwrap_or_else(|response| panic!("recovery lookup must begin: {response:?}"));
    assert_eq!(session.finish_definition(recovered, now).status(), QueryStatus::Ok);
}

#[test]
fn exact_path_position_and_active_revision_are_required() {
    let (mut session, revision, now) = semantic_session();
    for (id, path, offset) in [
        ("case", "SRC/main.zry", 47),
        ("parent", "../src/main.zry", 47),
        ("past-eof", "src/main.zry", 53),
    ] {
        let response = session
            .begin_definition(&definition_request(id, revision, 4, path, offset), now)
            .expect_err("invalid source coordinates must reject");
        assert_eq!(
            (response.status(), response.reason()),
            (QueryStatus::Malformed, Some(QueryReason::Source))
        );
    }

    let pending = session
        .begin_definition(&definition_request("stale", revision, 4, "src/main.zry", 47), now)
        .unwrap_or_else(|response| panic!("definition must begin: {response:?}"));
    session
        .admit_diagnostics(sources("src/main.zry", &SOURCE.replace('x', "y")), &[])
        .unwrap_or_else(|error| panic!("replacement revision must be admitted: {error}"));
    assert_eq!(session.finish_definition(pending, now).status(), QueryStatus::Stale);
}

#[test]
fn diagnostics_only_revision_reports_semantic_analysis_unavailable() {
    let mut session = DiagnosticSession::try_new()
        .unwrap_or_else(|error| panic!("session identity must be available: {error}"));
    let revision = session
        .admit_diagnostics(sources("src/main.zry", SOURCE), &[])
        .unwrap_or_else(|error| panic!("diagnostic revision must be admitted: {error}"));
    let now = Instant::now();
    let pending = session
        .begin_definition(&definition_request("unavailable", revision, 4, "src/main.zry", 47), now)
        .unwrap_or_else(|response| panic!("definition request must begin: {response:?}"));
    let response = session.finish_definition(pending, now);
    assert_eq!(
        (response.status(), response.reason()),
        (QueryStatus::Unavailable, Some(QueryReason::Analysis))
    );
}

#[test]
fn foreign_semantic_source_authority_is_rejected_before_retention() {
    let original = sources("src/main.zry", SOURCE);
    let syntax = verify_snapshot(raw_snapshot(), &original)
        .unwrap_or_else(|errors| panic!("definition fixture must authenticate: {errors:?}"));
    let foreign = sources("src/main.zry", SOURCE);
    let mut session = DiagnosticSession::try_new()
        .unwrap_or_else(|error| panic!("session identity must be available: {error}"));
    assert!(matches!(
        session.admit_semantics(foreign, &syntax),
        Err(crate::diagnostic_sessions::DiagnosticSessionError::SemanticAuthority)
    ));
    assert_eq!(session.active_revision(), None);
}

mod boundaries;

fn shifted_function(offset: u32, name: &str) -> RawFunctionSyntax {
    let mut function = raw_snapshot().files.remove(0).functions.remove(0);
    let shift = |range: &mut UntrustedSpan| {
        range.start += offset;
        range.end += offset;
    };
    function.name.text = name.to_owned();
    shift(&mut function.span);
    shift(&mut function.export_span);
    shift(&mut function.function_span);
    shift(&mut function.name.span);
    for parameter in &mut function.parameters {
        shift(&mut parameter.span);
        shift(&mut parameter.name.span);
        shift(&mut parameter.type_syntax.span);
    }
    shift(&mut function.result_type.span);
    shift(&mut function.body.span);
    for statement in &mut function.body.statements {
        shift(&mut statement.span);
        let RawStatementKind::Return { keyword_span, .. } = &mut statement.kind;
        shift(keyword_span);
    }
    for expression in &mut function.body.expressions {
        shift(&mut expression.span);
        if let RawExpressionKind::Reference { name } = &mut expression.kind {
            shift(&mut name.span);
        }
    }
    function
}

#[test]
fn same_named_parameters_resolve_within_their_own_functions() {
    let text = format!("{}{SOURCE}", SOURCE.replace("identity", "original"));
    let mut raw = raw_snapshot();
    raw.files[0].functions =
        vec![shifted_function(0, "original"), shifted_function(52, "identity")];
    let map = sources("src/main.zry", &text);
    let syntax = verify_snapshot(raw, &map)
        .unwrap_or_else(|errors| panic!("two-function fixture must authenticate: {errors:?}"));
    let mut session = DiagnosticSession::try_new().expect("valid definition fixture must succeed");
    let revision =
        session.admit_semantics(map, &syntax).expect("valid definition fixture must succeed");
    let now = Instant::now();
    for (offset, declaration) in [(47, 25), (99, 77)] {
        let pending = session
            .begin_definition(
                &definition_request("scope", revision, 100, "src/main.zry", offset),
                now,
            )
            .expect("valid definition fixture must succeed");
        let response = session.finish_definition(pending, now);
        assert_eq!(response.status(), QueryStatus::Ok);
        let value: Value = serde_json::from_str(
            response.encoded().expect("valid definition fixture must succeed"),
        )
        .expect("valid definition fixture must succeed");
        assert_eq!(
            value["result"]["locations"],
            json!([{
                "path": "src/main.zry", "byte_start": declaration, "byte_end": declaration + 1
            }])
        );
    }
}

#[test]
fn utf8_scalar_splits_reject_while_comments_and_eof_are_absent() {
    let text = format!("{SOURCE}// é x\n");
    let map = sources("src/main.zry", &text);
    let syntax =
        verify_snapshot(raw_snapshot(), &map).expect("valid definition fixture must succeed");
    let mut session = DiagnosticSession::try_new().expect("valid definition fixture must succeed");
    let revision =
        session.admit_semantics(map, &syntax).expect("valid definition fixture must succeed");
    let now = Instant::now();
    let split = session
        .begin_definition(
            &definition_request("coordinates", revision, 100, "src/main.zry", 56),
            now,
        )
        .expect_err("a byte inside a UTF-8 scalar must reject");
    assert_eq!(
        (split.status(), split.reason()),
        (QueryStatus::Malformed, Some(QueryReason::Source))
    );
    for offset in [55, 58, 60] {
        let pending = session
            .begin_definition(
                &definition_request("coordinates", revision, 100, "src/main.zry", offset),
                now,
            )
            .expect("valid definition fixture must succeed");
        let response = session.finish_definition(pending, now);
        assert_eq!(
            (response.status(), response.reason()),
            (QueryStatus::Absent, Some(QueryReason::Symbol))
        );
    }
    let pending = session
        .begin_definition(
            &definition_request("coordinates", revision, 100, "src/main.zry", 47),
            now,
        )
        .expect("valid definition fixture must succeed");
    assert_eq!(session.finish_definition(pending, now).status(), QueryStatus::Ok);
}

#[test]
fn rejected_semantics_do_not_replace_active_definition_authority() {
    let (mut session, revision, now) = semantic_session();
    let charge = session.cache_bytes();
    for rejection in ["unresolved", "duplicate", "type"] {
        let mut raw = raw_snapshot();
        let (text, code) = match rejection {
            "unresolved" => {
                if let RawExpressionKind::Reference { name } =
                    &mut raw.files[0].functions[0].body.expressions[0].kind
                {
                    name.text = "y".to_owned();
                }
                (SOURCE.replace("return x", "return y"), "ZRYNA-M1006")
            }
            "duplicate" => {
                raw.files[0].functions.push(shifted_function(52, "identity"));
                (SOURCE.repeat(2), "ZRYNA-M1001")
            }
            _ => {
                raw.files[0].functions[0].parameters[0].type_syntax.kind =
                    RawTypeSyntaxKind::Named { name: "any".to_owned() };
                (SOURCE.replacen("i32", "any", 1), "ZRYNA-M1004")
            }
        };
        let pending = session
            .begin_definition(
                &definition_request("preserved", revision, 100, "src/main.zry", 47),
                now,
            )
            .expect("valid definition fixture must succeed");
        let map = sources("src/main.zry", &text);
        let syntax = verify_snapshot(raw, &map).unwrap_or_else(|errors| {
            panic!("{rejection} must pass syntax verification: {errors:?}")
        });
        let error =
            session.admit_semantics(map, &syntax).expect_err("invalid semantics must reject");
        let crate::diagnostic_sessions::DiagnosticSessionError::Semantics(diagnostics) = error
        else {
            panic!("expected semantic rejection: {error:?}");
        };
        assert!(diagnostics.iter().any(|diagnostic| diagnostic.code == code));
        assert_eq!(session.active_revision(), Some(revision));
        assert_eq!(session.cache_bytes(), charge);
        assert_eq!(session.retained_revisions(), 1);
        assert_eq!(session.finish_definition(pending, now).status(), QueryStatus::Ok);
    }
}
