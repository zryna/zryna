use std::io::Cursor;

use super::*;

fn session(protocol: u32, text: &str) -> Vec<Value> {
    let request = json!({"id":2,"method":"analyze","params":{
        "schema_version":protocol,"files":[{"path":"src/main.zry","text":text}]}});
    let input = format!("{{\"id\":1,\"method\":\"handshake\"}}\n{request}\n");
    let mut output = Vec::new();
    run(protocol, &mut Cursor::new(input), &mut output).expect("native session");
    output
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).expect("complete JSON frame"))
        .collect()
}

#[test]
fn native_worker_all_versions_produce_only_original_verifier_admitted_candidates() {
    let text = "// π\r\nexport function answer(): i32 { return 42; }\r\n";
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "src/main.zry".to_owned(),
        text: text.to_owned(),
    }])
    .expect("same authoritative bytes");
    for protocol in 2..=4 {
        let responses = session(protocol, text);
        assert_eq!(responses.len(), 2);
        assert_eq!(responses[0]["result"]["provider"], NATIVE_PROVIDER);
        assert_eq!(responses[0]["result"]["protocol_version"], protocol);
        assert_eq!(responses[0]["result"]["capabilities"]["module_resolution"], false);
        let value = responses[1]["result"].clone();
        match protocol {
            2 => {
                syntax_v2::verify_snapshot(
                    serde_json::from_value(value).expect("original protocol candidate DTO"),
                    &sources,
                )
                .expect("original v2 verifier");
            }
            3 => {
                syntax_v3::verify_snapshot(
                    serde_json::from_value(value).expect("original protocol candidate DTO"),
                    &sources,
                )
                .expect("original v3 verifier");
            }
            4 => {
                syntax_v4::verify_snapshot(
                    serde_json::from_value(value).expect("original protocol candidate DTO"),
                    &sources,
                )
                .expect("original v4 verifier");
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn native_worker_fatal_lexical_errors_are_rpc_rejections_with_original_owner_diagnostics() {
    let text = "export function answer(): i32 { return `forbidden`; }";
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "src/main.zry".to_owned(),
        text: text.to_owned(),
    }])
    .expect("authoritative fixture source map");
    let lexed = native_lexer::lex(&sources).expect("fixture lexes under original limits");
    for protocol in 2..=4 {
        let diagnostic = match protocol {
            2 => native_parser::parse_v2_recovering_candidate(&sources, &lexed)
                .expect_err("fatal lexical source")
                .diagnostic()
                .clone(),
            3 => native_parser::v3::parse_v3_straight_line_candidate(&sources, &lexed)
                .expect_err("fatal lexical source")
                .diagnostic()
                .clone(),
            4 => native_parser::v4::parse_v4_candidate(&sources, &lexed)
                .expect_err("fatal lexical source")
                .diagnostic()
                .clone(),
            _ => unreachable!(),
        };
        let responses = session(protocol, text);
        assert_eq!(responses[1], rejection(&diagnostic));
        assert!(responses[1].get("result").is_none());
    }
}

#[test]
fn native_worker_unsupported_import_rejection_keeps_the_original_source_span() {
    let frames = session(3, "import \"./dep.zry\";\n");
    assert_eq!(
        frames[1],
        json!({"id":2,"error":{"code":"ZRYNA-F2002","message":
            "import declaration uses unsupported syntax 'ImportDeclaration' at file 0 bytes 0..19"}})
    );
}

#[test]
fn native_worker_recoverable_diagnostics_preserve_the_entire_original_result() {
    let text = "export const rejected = 1; export function answer(): i32 { return 42; }";
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "src/main.zry".to_owned(),
        text: text.to_owned(),
    }])
    .expect("authoritative fixture source map");
    let lexed = native_lexer::lex(&sources).expect("fixture lexes under original limits");
    let raw = native_parser::parse_v2_recovering_candidate(&sources, &lexed)
        .expect("recoverable v2 candidate");
    assert!(!raw.diagnostics.is_empty());
    let responses = session(2, text);
    assert_eq!(responses[1], json!({"id":2,"result":raw}));
    assert!(responses[1].get("error").is_none());
}

#[test]
fn native_worker_preserves_hostile_raw_candidates_for_the_original_verifier() {
    let text = include_str!("../../../../tests/m3-fixtures/borrow-exclusive-nonreference.zry");
    let sources = SourceMap::build(vec![SourceFileInput {
        path: "src/main.zry".to_owned(),
        text: text.to_owned(),
    }])
    .expect("authoritative fixture source map");
    let lexed = native_lexer::lex(&sources).expect("fixture lexes under original limits");
    let raw =
        native_parser::v4::parse_v4_candidate(&sources, &lexed).expect("raw v4 hostile candidate");
    let original = serde_json::to_value(&raw).expect("entire raw candidate serialization");
    let errors = syntax_v4::verify_snapshot(raw, &sources)
        .expect_err("original verifier rejects hostile operand");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code(), "ZRYNA-Y4002");
    let responses = session(4, text);
    assert_eq!(responses[1], json!({"id":2,"result":original}));
}

#[test]
fn native_worker_rejects_analysis_before_handshake_and_unknown_handshake_fields() {
    for input in [
        "{\"id\":2,\"method\":\"analyze\",\"params\":{\"schema_version\":2,\"files\":[]}}\n",
        "{\"id\":1,\"method\":\"handshake\",\"params\":null}\n",
        "{\"id\":1,\"method\":\"handshake\",\"ambient_root\":\"/\"}\n",
        "{\"id\":99,\"method\":\"handshake\"}\n",
    ] {
        let mut output = Vec::new();
        assert!(run(2, &mut Cursor::new(input), &mut output).is_err());
        assert!(output.is_empty());
    }
}

#[test]
fn native_worker_rejects_foreign_paths_wrong_protocol_and_additional_requests() {
    for params in [
        json!({"schema_version":3,"files":[]}),
        json!({"schema_version":2,"files":[{"path":"../foreign.zry","text":""}]}),
        json!({"schema_version":2,"files":[],"grants":["network"]}),
    ] {
        let input = format!(
            "{{\"id\":1,\"method\":\"handshake\"}}\n{}\n",
            json!({"id":2,"method":"analyze","params":params})
        );
        let mut output = Vec::new();
        assert!(run(2, &mut Cursor::new(input), &mut output).is_err());
        let frames = output.split(|byte| *byte == b'\n').collect::<Vec<_>>();
        assert_eq!(frames.len(), 2, "only one complete handshake frame");
        assert!(frames[1].is_empty(), "handshake ends with its delimiter");
    }
    let request = json!({"id":2,"method":"analyze","params":{
        "schema_version":2,"files":[{"path":"src/main.zry","text":
            "export function answer(): i32 { return 42; }"}]}});
    let input = format!("{{\"id\":1,\"method\":\"handshake\"}}\n{request}\n{request}\n");
    assert!(run(2, &mut Cursor::new(input), &mut Vec::new()).is_err());
}

#[test]
fn native_worker_first_extra_framing_and_response_bytes_fail_without_partial_output() {
    let mut input = Cursor::new(b"0123456789\nremaining".to_vec());
    assert!(request(&mut input, 8).is_err());
    assert_eq!(input.position(), 9);
    assert!(request(&mut Cursor::new(b"{}"), 8).is_err());
    let mut output = Vec::new();
    assert!(respond(&mut output, &"12345", 6).is_err());
    assert!(output.is_empty());
    respond(&mut output, &"12345", 7).expect("exact response-byte boundary");
    assert_eq!(output, b"\"12345\"\n");
}

#[test]
fn native_worker_source_count_is_checked_during_decode() {
    let params = json!({"schema_version":2,"files":vec![
        json!({"path":"a.zry","text":""}); MAX_SOURCE_FILES + 1]});
    assert!(serde_json::from_value::<Analysis>(params).is_err());
    assert!(run(1, &mut Cursor::new(Vec::<u8>::new()), &mut Vec::new()).is_err());
}

#[test]
fn provider_comparison_failure_has_the_existing_structural_rejection_category() {
    let error = crate::WorkerError::provider_divergence();
    assert_eq!(error.failure(), crate::WorkerFailure::SnapshotVerification);
    assert_eq!(error.code(), "ZRYNA-F1107");
    assert_eq!(error.diagnostics().len(), 1);
    assert!(error.diagnostics()[0].primary_span().is_none());
    assert!(error.diagnostics()[0].message().contains("observations differ"));
}
