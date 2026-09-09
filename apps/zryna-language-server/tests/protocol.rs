//! Protocol coverage for revision, validation, and lifecycle behavior.

use serde_json::{Value, json};
use zryna_driver::diagnostic_sessions::{
    DiagnosticRevision, DiagnosticSession, DiagnosticSessionError, admit_single_function_fixture,
};
use zryna_language_server::{RevisionCompiler, Server, read_frame, write_frame};
use zryna_source::SourceMap;

struct EmptyCompiler;

struct SemanticCompiler;

impl RevisionCompiler for EmptyCompiler {
    type Error = DiagnosticSessionError;

    fn admit(
        &mut self,
        session: &mut DiagnosticSession,
        sources: SourceMap,
    ) -> Result<DiagnosticRevision, Self::Error> {
        session.admit_diagnostics(sources, &[])
    }
}

impl RevisionCompiler for SemanticCompiler {
    type Error = DiagnosticSessionError;

    fn admit(
        &mut self,
        session: &mut DiagnosticSession,
        sources: SourceMap,
    ) -> Result<DiagnosticRevision, Self::Error> {
        admit_single_function_fixture(session, sources)
    }
}

fn request(server: &mut Server<impl RevisionCompiler>, value: impl serde::Serialize) -> Vec<Value> {
    let bytes = serde_json::to_vec(&value).unwrap_or_else(|error| panic!("fixture: {error}"));
    server.handle_bytes(&bytes)
}

fn initialized() -> Server<EmptyCompiler> {
    let mut server = Server::new(EmptyCompiler).unwrap_or_else(|error| panic!("session: {error}"));
    let output = request(
        &mut server,
        json!({
            "jsonrpc":"2.0","id":1,"method":"initialize","params":{
                "rootUri":"file:///workspace",
                "capabilities":{"general":{"positionEncodings":["utf-8","utf-16"]}}
            }
        }),
    );
    assert_eq!(output.len(), 1);
    assert_eq!(output[0]["result"]["capabilities"]["positionEncoding"], "utf-8");
    assert!(
        request(&mut server, json!({"jsonrpc":"2.0","method":"initialized","params":{}}))
            .is_empty()
    );
    server
}

fn semantic_initialized() -> Server<SemanticCompiler> {
    let mut server =
        Server::new(SemanticCompiler).unwrap_or_else(|error| panic!("session: {error}"));
    let output = request(
        &mut server,
        json!({
            "jsonrpc":"2.0","id":1,"method":"initialize","params":{
                "rootUri":"file:///workspace",
                "capabilities":{"general":{"positionEncodings":["utf-8"]}}
            }
        }),
    );
    assert_eq!(output.len(), 1);
    let _ = request(&mut server, json!({"jsonrpc":"2.0","method":"initialized","params":{}}));
    server
}

fn open<Compiler: RevisionCompiler>(
    server: &mut Server<Compiler>,
    version: i64,
    text: &str,
) -> Vec<Value> {
    request(
        server,
        json!({
            "jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{
                "uri":"file:///workspace/src/main.zry","languageId":"zryna","version":version,"text":text
            }}
        }),
    )
}

fn change<Compiler: RevisionCompiler>(
    server: &mut Server<Compiler>,
    version: i64,
    text: &str,
) -> Vec<Value> {
    request(
        server,
        json!({
            "jsonrpc":"2.0","method":"textDocument/didChange","params":{
                "textDocument":{"uri":"file:///workspace/src/main.zry","version":version},
                "contentChanges":[{"text":text}]
            }
        }),
    )
}

fn definition<Compiler: RevisionCompiler>(
    server: &mut Server<Compiler>,
    id: i64,
    line: u32,
    character: u32,
) -> Vec<Value> {
    request(
        server,
        json!({
            "jsonrpc":"2.0","id":id,"method":"textDocument/definition","params":{
                "textDocument":{"uri":"file:///workspace/src/main.zry"},
                "position":{"line":line,"character":character}
            }
        }),
    )
}

#[test]
fn framing_is_exact_bounded_and_recovers_between_frames() {
    let first = br#"{"jsonrpc":"2.0"}"#;
    let second = br#"{"jsonrpc":"2.0","method":"exit"}"#;
    let mut wire = Vec::new();
    write_frame(&mut wire, first).unwrap_or_else(|error| panic!("frame: {error}"));
    write_frame(&mut wire, second).unwrap_or_else(|error| panic!("frame: {error}"));
    let mut input = std::io::BufReader::new(wire.as_slice());
    assert_eq!(read_frame(&mut input).expect("first frame"), Some(first.to_vec()));
    assert_eq!(read_frame(&mut input).expect("second frame"), Some(second.to_vec()));
    assert_eq!(read_frame(&mut input).expect("clean eof"), None);

    let mut duplicate =
        std::io::BufReader::new(b"Content-Length: 0\r\nContent-Length: 0\r\n\r\n".as_slice());
    assert!(read_frame(&mut duplicate).is_err());

    let oversized =
        format!("Content-Length: {}\r\n\r\n", zryna_language_server::MAX_LSP_MESSAGE_BYTES + 1);
    assert!(read_frame(&mut std::io::BufReader::new(oversized.as_bytes())).is_err());
    let mut exact = vec![0_u8; zryna_language_server::MAX_LSP_MESSAGE_BYTES];
    write_frame(&mut std::io::sink(), &exact).expect("exact frame limit");
    exact.push(0);
    assert!(write_frame(&mut std::io::sink(), &exact).is_err());
}

#[test]
fn open_change_and_close_publish_revision_bound_diagnostics() {
    let mut server = initialized();
    let first = open(&mut server, 1, "export function a(x: i32): i32 { return x; }\n");
    assert_eq!(first.len(), 2);
    assert_eq!(first[0]["method"], "zryna/publishDiagnostics");
    assert_eq!(first[0]["params"]["revision"], 1);
    assert_eq!(first[0]["params"]["documents"][0]["version"], 1);
    assert_eq!(first[1]["method"], "textDocument/publishDiagnostics");

    let second = change(&mut server, 2, "export function b(x: i32): i32 { return x; }\n");
    assert_eq!(second[0]["params"]["revision"], 2);
    assert_ne!(first[0]["params"]["snapshot"], second[0]["params"]["snapshot"]);

    let stale = change(&mut server, 2, "same version");
    assert_eq!(stale[0]["method"], "window/logMessage");
    let closed = request(
        &mut server,
        json!({
            "jsonrpc":"2.0","method":"textDocument/didClose","params":{
                "textDocument":{"uri":"file:///workspace/src/main.zry"}
            }
        }),
    );
    assert_eq!(closed[0]["params"]["diagnostics"], json!([]));
}

#[test]
fn queued_definition_is_cancelled_and_next_request_recovers() {
    let mut server = semantic_initialized();
    let _ = open(&mut server, 1, "export function a(x: i32): i32 { return x; }\n");
    assert!(definition(&mut server, 7, 0, 40).is_empty());
    assert!(
        request(
            &mut server,
            json!({
                "jsonrpc":"2.0","method":"$/cancelRequest","params":{"id":7}
            })
        )
        .is_empty()
    );
    let cancelled = server.finish_pending();
    assert_eq!(cancelled[0]["error"]["code"], -32800);

    assert!(definition(&mut server, 8, 0, 40).is_empty());
    let recovered = server.finish_pending();
    assert_eq!(recovered[0]["result"]["range"]["start"]["character"], 18);
}

#[test]
fn same_length_change_suppresses_stale_definition() {
    let mut server = semantic_initialized();
    let _ = open(&mut server, 1, "export function a(x: i32): i32 { return x; }\n");
    assert!(definition(&mut server, 9, 0, 40).is_empty());
    let _ = change(&mut server, 2, "export function a(y: i32): i32 { return y; }\n");
    let stale = server.finish_pending();
    assert_eq!(stale[0]["error"]["code"], -32801);
}

#[test]
fn unicode_positions_reject_split_scalars_and_recover() {
    let mut server = initialized();
    let text = "// é😀e\u{301}\r\nexport function identity(x: i32): i32 { return x; }\n";
    let _ = open(&mut server, 1, text);
    let split = definition(&mut server, 10, 0, 4);
    assert_eq!(split[0]["error"]["code"], -32602);
    assert!(definition(&mut server, 11, 1, 47).is_empty());
    assert_eq!(server.finish_pending()[0]["error"]["code"], -32803);
}

#[test]
fn malformed_foreign_and_unsupported_messages_cannot_publish() {
    let mut server = initialized();
    let foreign = request(
        &mut server,
        json!({
            "jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{
                "uri":"file:///elsewhere/main.zry","languageId":"zryna","version":1,"text":"x"
            }}
        }),
    );
    assert_eq!(foreign[0]["method"], "window/logMessage");
    let traversal = request(
        &mut server,
        json!({
            "jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{
                "uri":"file:///workspace/../main.zry","languageId":"zryna","version":1,"text":"x"
            }}
        }),
    );
    assert_eq!(traversal[0]["method"], "window/logMessage");
    let malformed =
        server.handle_bytes(br#"{"jsonrpc":"2.0","id":1,"id":2,"method":"initialize"}"#);
    assert_eq!(malformed[0]["error"]["code"], -32700);
    let nested_duplicate = server.handle_bytes(
        br#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///workspace/a.zry","uri":"file:///workspace/b.zry","languageId":"zryna","version":1,"text":"x"}}}"#,
    );
    assert_eq!(nested_duplicate[0]["error"]["code"], -32700);
    let nested = format!("{}0{}", "[".repeat(65), "]".repeat(65));
    let deep = format!("{{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"x\",\"params\":{nested} }}");
    let over_depth = server.handle_bytes(deep.as_bytes());
    assert_eq!(over_depth[0]["error"]["code"], -32600);
    let unsupported = request(
        &mut server,
        json!({
            "jsonrpc":"2.0","id":"x","method":"textDocument/formatting","params":{}
        }),
    );
    assert_eq!(unsupported[0]["error"]["code"], -32601);
    let recovered = open(&mut server, 1, "export function a(x: i32): i32 { return x; }\n");
    assert_eq!(recovered[0]["method"], "zryna/publishDiagnostics");
}

#[test]
fn shutdown_requires_ordered_exit() {
    let mut server = initialized();
    let shutdown = request(&mut server, json!({"jsonrpc":"2.0","id":12,"method":"shutdown"}));
    assert!(shutdown[0]["result"].is_null());
    assert!(!server.should_exit());
    assert!(request(&mut server, json!({"jsonrpc":"2.0","method":"exit"})).is_empty());
    assert!(server.should_exit());
}
